use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::{agent::AgentKind, infrastructure::CliRunner};

struct TerminalSession {
    runner: Arc<CliRunner>,
    agent: AgentKind,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    watcher: JoinHandle<()>,
}

#[derive(Clone)]
pub struct TerminalStatus {
    pub id: String,
    pub runner: Arc<CliRunner>,
    pub agent: AgentKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub chat_id: i64,
    pub agent: AgentKind,
    pub workspace: PathBuf,
    pub tmux_session: String,
    pub created_at: u64,
    pub status: SessionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Running,
    Stopped,
    Missing,
}

#[derive(Default, Serialize, Deserialize)]
struct StoredSessions {
    sessions: Vec<SessionRecord>,
    current: HashMap<i64, String>,
}

/// Owns the lifecycle and chat binding of all persistent tmux terminals.
pub struct TerminalManager {
    home: PathBuf,
    state_file: PathBuf,
    sessions: Mutex<HashMap<String, TerminalSession>>,
    records: Mutex<StoredSessions>,
}

impl TerminalManager {
    pub fn new(home: PathBuf) -> Self {
        let state_file = home.join(".chatcli").join("sessions.json");
        let records = fs::read_to_string(&state_file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self {
            home,
            state_file,
            sessions: Mutex::new(HashMap::new()),
            records: Mutex::new(records),
        }
    }

    pub fn home(&self) -> &PathBuf {
        &self.home
    }

    /// Rebuild in-memory runners after a ChatCLI restart and verify tmux still exists.
    pub async fn restore(&self) {
        let candidates = self.records.lock().await.sessions.clone();
        for record in candidates
            .into_iter()
            .filter(|record| record.status == SessionState::Running)
        {
            let runner = Arc::new(CliRunner::new(
                record.workspace.clone(),
                record.tmux_session.clone(),
            ));
            if runner.exists().await {
                self.sessions.lock().await.insert(
                    record.id,
                    TerminalSession {
                        runner,
                        agent: record.agent,
                        stop: None,
                        watcher: tokio::spawn(async {}),
                    },
                );
            } else {
                self.mark_missing(&record.id).await;
            }
        }
    }

    pub fn session_name(&self, chat_id: i64, id: &str) -> String {
        format!("chatcli-tg-{}-{id}", chat_id.unsigned_abs())
    }

    pub async fn active(&self, chat_id: i64) -> Option<TerminalStatus> {
        let id = self.records.lock().await.current.get(&chat_id).cloned()?;
        self.sessions
            .lock()
            .await
            .get(&id)
            .map(|session| TerminalStatus {
                id,
                runner: Arc::clone(&session.runner),
                agent: session.agent,
            })
    }

    pub async fn create(
        &self,
        chat_id: i64,
        id: String,
        runner: Arc<CliRunner>,
        agent: AgentKind,
        stop: tokio::sync::oneshot::Sender<()>,
        watcher: JoinHandle<()>,
    ) {
        let record = SessionRecord {
            id: id.clone(),
            chat_id,
            agent,
            workspace: runner.workspace.clone(),
            tmux_session: runner.tmux_session.clone(),
            created_at: now_millis(),
            status: SessionState::Running,
        };
        self.sessions.lock().await.insert(
            id.clone(),
            TerminalSession {
                runner,
                agent,
                stop: Some(stop),
                watcher,
            },
        );
        let mut records = self.records.lock().await;
        records.sessions.push(record);
        records.current.insert(chat_id, id);
        self.save(&records);
    }

    pub async fn list(&self, chat_id: i64, agent: AgentKind) -> Vec<SessionRecord> {
        let records = self.records.lock().await;
        let mut out = records
            .sessions
            .iter()
            .filter(|record| record.chat_id == chat_id && record.agent == agent)
            .cloned()
            .collect::<Vec<_>>();
        out.sort_by_key(|record| std::cmp::Reverse(record.created_at));
        out
    }

    pub async fn select(&self, chat_id: i64, id: &str) -> bool {
        let record = self
            .records
            .lock()
            .await
            .sessions
            .iter()
            .find(|record| {
                record.chat_id == chat_id
                    && record.id == id
                    && record.status != SessionState::Stopped
            })
            .cloned();
        let Some(record) = record else {
            return false;
        };
        if !self.sessions.lock().await.contains_key(id) {
            let runner = Arc::new(CliRunner::new(record.workspace, record.tmux_session));
            if !runner.exists().await {
                self.mark_missing(id).await;
                return false;
            }
            // Restored sessions can be interacted with immediately; terminal streaming resumes
            // when the user opens a fresh session in this process.
            self.sessions.lock().await.insert(
                id.to_owned(),
                TerminalSession {
                    runner,
                    agent: record.agent,
                    stop: None,
                    watcher: tokio::spawn(async {}),
                },
            );
        }
        let mut records = self.records.lock().await;
        if let Some(record) = records.sessions.iter_mut().find(|record| record.id == id) {
            record.status = SessionState::Running;
        }
        records.current.insert(chat_id, id.to_owned());
        self.save(&records);
        true
    }

    pub async fn clear_current(&self, chat_id: i64) {
        let mut records = self.records.lock().await;
        records.current.remove(&chat_id);
        self.save(&records);
    }

    pub async fn close_current(&self, chat_id: i64) -> bool {
        let id = self.records.lock().await.current.get(&chat_id).cloned();
        match id {
            Some(id) => self.close(&id).await,
            None => false,
        }
    }

    pub async fn close(&self, id: &str) -> bool {
        let session = self.sessions.lock().await.remove(id);
        let Some(mut session) = session else {
            self.mark_stopped(id).await;
            return false;
        };
        if let Some(stop) = session.stop.take() {
            let _ = stop.send(());
        }
        session.watcher.abort();
        let _ = session.runner.kill().await;
        self.mark_stopped(id).await;
        true
    }

    async fn mark_stopped(&self, id: &str) {
        let mut records = self.records.lock().await;
        if let Some(record) = records.sessions.iter_mut().find(|record| record.id == id) {
            record.status = SessionState::Stopped;
        }
        records.current.retain(|_, current| current != id);
        self.save(&records);
    }

    async fn mark_missing(&self, id: &str) {
        let mut records = self.records.lock().await;
        if let Some(record) = records.sessions.iter_mut().find(|record| record.id == id) {
            record.status = SessionState::Missing;
        }
        records.current.retain(|_, current| current != id);
        self.save(&records);
    }

    fn save(&self, records: &StoredSessions) {
        if let Some(parent) = self.state_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_vec_pretty(records) {
            let _ = fs::write(&self.state_file, json);
        }
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

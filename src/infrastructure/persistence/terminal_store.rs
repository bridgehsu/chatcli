use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, task::JoinHandle};
use tracing::warn;

use crate::{domain::TerminalKind, infrastructure::CliRunner};

struct TerminalSession {
    runner: Arc<CliRunner>,
    kind: TerminalKind,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    watcher: JoinHandle<()>,
}

#[derive(Clone)]
pub struct TerminalStatus {
    pub id: String,
    pub runner: Arc<CliRunner>,
    pub kind: TerminalKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub chat_id: i64,
    /// 所属的 Agent Session；一个 Agent Session 可拥有多个 Terminal Session。
    #[serde(default)]
    pub agent_session_id: String,
    /// 旧状态文件使用 `agent` 字段，读取时兼容为新的终端类型字段。
    #[serde(alias = "agent")]
    pub kind: TerminalKind,
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
    /// 每个 Agent Session 当前正在交互的 Terminal Session ID。
    current: HashMap<String, String>,
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
                        kind: record.kind,
                        stop: None,
                        watcher: tokio::spawn(async {}),
                    },
                );
            } else {
                self.mark_missing(&record.id).await;
            }
        }
    }

    /// 兼容旧版按 chat_id 保存的终端记录。
    ///
    /// 新版将 tmux 终端归属到 Agent Session；升级时把该 chat 下尚未归属的记录
    /// 迁移到启动后恢复出的当前 Agent Session。
    pub async fn migrate_legacy_chat(&self, chat_id: i64, agent_session_id: &str) {
        let mut records = self.records.lock().await;
        let mut changed = false;
        for record in records
            .sessions
            .iter_mut()
            .filter(|record| record.chat_id == chat_id && record.agent_session_id.is_empty())
        {
            record.agent_session_id = agent_session_id.to_owned();
            changed = true;
        }

        let old_current = records.current.remove(&chat_id.to_string());
        if let Some(terminal_id) = old_current {
            records
                .current
                .insert(agent_session_id.to_owned(), terminal_id);
            changed = true;
        }
        if changed {
            self.save(&records);
        }
    }

    pub fn session_name(&self, chat_id: i64, id: &str) -> String {
        format!("chatcli-tg-{}-{id}", chat_id.unsigned_abs())
    }

    pub async fn active(&self, agent_session_id: &str) -> Option<TerminalStatus> {
        let id = self
            .records
            .lock()
            .await
            .current
            .get(agent_session_id)
            .cloned()?;
        let status = self
            .sessions
            .lock()
            .await
            .get(&id)
            .map(|session| TerminalStatus {
                id: id.clone(),
                runner: Arc::clone(&session.runner),
                kind: session.kind,
            });
        let Some(status) = status else {
            return None;
        };
        if status.runner.exists().await {
            Some(status)
        } else {
            warn!(terminal_id = %id, "Active tmux terminal is no longer available");
            self.mark_missing(&id).await;
            None
        }
    }

    pub async fn create(
        &self,
        chat_id: i64,
        agent_session_id: String,
        id: String,
        runner: Arc<CliRunner>,
        kind: TerminalKind,
        stop: tokio::sync::oneshot::Sender<()>,
        watcher: JoinHandle<()>,
    ) {
        let record = SessionRecord {
            id: id.clone(),
            chat_id,
            agent_session_id: agent_session_id.clone(),
            kind,
            workspace: runner.workspace.clone(),
            tmux_session: runner.tmux_session.clone(),
            created_at: now_millis(),
            status: SessionState::Running,
        };
        self.sessions.lock().await.insert(
            id.clone(),
            TerminalSession {
                runner,
                kind,
                stop: Some(stop),
                watcher,
            },
        );
        let mut records = self.records.lock().await;
        records.sessions.push(record);
        records.current.insert(agent_session_id, id);
        self.save(&records);
    }

    pub async fn list(&self, agent_session_id: &str) -> Vec<SessionRecord> {
        let records = self.records.lock().await;
        let mut out = records
            .sessions
            .iter()
            .filter(|record| record.agent_session_id == agent_session_id)
            .cloned()
            .collect::<Vec<_>>();
        out.sort_by_key(|record| std::cmp::Reverse(record.created_at));
        out
    }

    pub async fn select(&self, agent_session_id: &str, id: &str) -> bool {
        let record = self
            .records
            .lock()
            .await
            .sessions
            .iter()
            .find(|record| {
                record.agent_session_id == agent_session_id
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
            // when the user opens a fresh manager in this process.
            self.sessions.lock().await.insert(
                id.to_owned(),
                TerminalSession {
                    runner,
                    kind: record.kind,
                    stop: None,
                    watcher: tokio::spawn(async {}),
                },
            );
        }
        let mut records = self.records.lock().await;
        if let Some(record) = records.sessions.iter_mut().find(|record| record.id == id) {
            record.status = SessionState::Running;
        }
        records
            .current
            .insert(agent_session_id.to_owned(), id.to_owned());
        self.save(&records);
        true
    }

    pub async fn clear_current(&self, agent_session_id: &str) {
        let mut records = self.records.lock().await;
        records.current.remove(agent_session_id);
        self.save(&records);
    }

    pub async fn close_current(&self, agent_session_id: &str) -> bool {
        let id = self
            .records
            .lock()
            .await
            .current
            .get(agent_session_id)
            .cloned();
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

    /// 停止并删除一个 chat 追踪的全部 Terminal Session。
    pub async fn reset_chat(&self, chat_id: i64) {
        let (terminal_ids, agent_session_ids) = {
            let records = self.records.lock().await;
            let terminal_ids = records
                .sessions
                .iter()
                .filter(|record| record.chat_id == chat_id)
                .map(|record| record.id.clone())
                .collect::<Vec<_>>();
            let agent_session_ids = records
                .sessions
                .iter()
                .filter(|record| record.chat_id == chat_id)
                .map(|record| record.agent_session_id.clone())
                .collect::<HashSet<_>>();
            (terminal_ids, agent_session_ids)
        };
        for terminal_id in &terminal_ids {
            self.close(terminal_id).await;
        }
        let terminal_ids = terminal_ids.into_iter().collect::<HashSet<_>>();
        let mut records = self.records.lock().await;
        records.sessions.retain(|record| record.chat_id != chat_id);
        records.current.retain(|session_id, terminal_id| {
            !agent_session_ids.contains(session_id) && !terminal_ids.contains(terminal_id)
        });
        self.save(&records);
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

//! Agent Session 快照的本地 JSON 持久化。

use crate::domain::{
    AgentKind, AgentSessionState as SessionState, PendingLaunch, PendingShellCommand, Session,
    SessionStatus,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;

#[derive(Default, Serialize, Deserialize)]
struct StoredSessions {
    sessions: Vec<Session>,
    current: HashMap<i64, String>,
}

/// 管理 Session 快照的读取、创建、切换与写回。
pub struct SessionStore {
    state_file: PathBuf,
    sessions: Mutex<StoredSessions>,
}

impl SessionStore {
    pub fn new(home: &PathBuf) -> Self {
        let state_file = home.join(".chatcli").join("agent-sessions.json");
        let sessions = fs::read_to_string(&state_file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self {
            state_file,
            sessions: Mutex::new(sessions),
        }
    }

    pub async fn restore(&self) -> HashMap<i64, Session> {
        let stored = self.sessions.lock().await;
        stored
            .current
            .iter()
            .filter_map(|(chat, id)| {
                stored
                    .sessions
                    .iter()
                    .find(|session| session.id == *id && session.status == SessionStatus::Active)
                    .cloned()
                    .map(|session| (*chat, session))
            })
            .collect()
    }

    pub async fn list(&self, chat_id: i64) -> (Vec<Session>, Option<String>) {
        let stored = self.sessions.lock().await;
        let mut sessions = stored
            .sessions
            .iter()
            .filter(|session| session.chat_id == chat_id)
            .cloned()
            .collect::<Vec<_>>();
        sessions.sort_by_key(|session| std::cmp::Reverse(session.updated));
        (sessions, stored.current.get(&chat_id).cloned())
    }

    pub async fn current(&self, chat_id: i64) -> Option<Session> {
        let stored = self.sessions.lock().await;
        let id = stored.current.get(&chat_id)?;
        stored
            .sessions
            .iter()
            .find(|session| session.id == *id && session.status == SessionStatus::Active)
            .cloned()
    }

    pub async fn ensure_current(&self, chat_id: i64) -> Session {
        let mut stored = self.sessions.lock().await;
        let id = current_or_create(&mut stored, chat_id);
        let session = stored
            .sessions
            .iter()
            .find(|session| session.id == id)
            .cloned()
            .expect("current_or_create must create a session");
        self.save(&stored);
        session
    }

    pub async fn select(&self, chat_id: i64, id: &str) -> Option<Session> {
        let mut stored = self.sessions.lock().await;
        let session = stored
            .sessions
            .iter()
            .find(|session| {
                session.id == id
                    && session.chat_id == chat_id
                    && session.status == SessionStatus::Active
            })
            .cloned()?;
        stored.current.insert(chat_id, id.to_owned());
        self.save(&stored);
        Some(session)
    }

    pub async fn close_current(&self, chat_id: i64) -> Option<Session> {
        let mut stored = self.sessions.lock().await;
        let id = stored.current.remove(&chat_id)?;
        let session = stored
            .sessions
            .iter_mut()
            .find(|session| session.id == id)?;
        session.status = SessionStatus::Closed;
        session.updated = now_millis();
        let closed = session.clone();
        self.save(&stored);
        Some(closed)
    }

    pub async fn new_session(&self, chat_id: i64) {
        let mut stored = self.sessions.lock().await;
        let now = now_millis();
        let id = format!("agent-{chat_id}-{now}");
        stored.sessions.push(Session {
            id: id.clone(),
            chat_id,
            state: SessionState::Initial,
            status: SessionStatus::Active,
            created: now,
            updated: now,
        });
        stored.current.insert(chat_id, id);
        self.save(&stored);
    }

    pub async fn clear_chat(&self, chat_id: i64) {
        let mut stored = self.sessions.lock().await;
        stored.sessions.retain(|session| session.chat_id != chat_id);
        stored.current.remove(&chat_id);
        self.save(&stored);
    }

    pub async fn save_state(
        &self,
        chat_id: i64,
        selected: Option<AgentKind>,
        pending: Option<PendingLaunch>,
        pending_shell: Option<PendingShellCommand>,
        terminal_session_id: Option<String>,
    ) {
        let state = match pending_shell {
            Some(command) => SessionState::WaitingShellCommand {
                terminal_session_id: command.terminal_session_id,
                command: command.command,
                description: command.description,
            },
            None => match pending {
                Some(pending) => SessionState::WaitingWorkspace {
                    kind: pending.kind,
                    candidates: pending.candidates,
                    workspace: pending.workspace,
                },
                None => terminal_session_id
                    .map(|id| SessionState::ActiveTerminal {
                        terminal_session_id: id,
                    })
                    .or_else(|| selected.map(|kind| SessionState::CliSelected { kind }))
                    .unwrap_or(SessionState::Initial),
            },
        };
        let mut stored = self.sessions.lock().await;
        let id = current_or_create(&mut stored, chat_id);
        if let Some(session) = stored.sessions.iter_mut().find(|session| session.id == id) {
            session.state = state;
            session.updated = now_millis();
        }
        self.save(&stored);
    }

    fn save(&self, stored: &StoredSessions) {
        if let Some(parent) = self.state_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_vec_pretty(stored) {
            let _ = fs::write(&self.state_file, json);
        }
    }
}

fn current_or_create(stored: &mut StoredSessions, chat_id: i64) -> String {
    if let Some(id) = stored.current.get(&chat_id) {
        return id.clone();
    }
    let now = now_millis();
    let id = format!("agent-{chat_id}-{now}");
    stored.sessions.push(Session {
        id: id.clone(),
        chat_id,
        state: SessionState::Initial,
        status: SessionStatus::Active,
        created: now,
        updated: now,
    });
    stored.current.insert(chat_id, id.clone());
    id
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use tokio::{sync::Mutex, task::JoinHandle};

use crate::{agent::AgentKind, cli::CliRunner};

struct TerminalSession {
    runner: Arc<CliRunner>,
    agent: AgentKind,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    watcher: JoinHandle<()>,
}

#[derive(Clone)]
pub struct TerminalStatus {
    pub runner: Arc<CliRunner>,
    pub agent: AgentKind,
}

/// Owns the lifecycle and chat binding of all persistent tmux terminals.
pub struct TerminalManager {
    home: PathBuf,
    sessions: Mutex<HashMap<i64, TerminalSession>>,
}

impl TerminalManager {
    pub fn new(home: PathBuf) -> Self {
        Self {
            home,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn home(&self) -> &PathBuf {
        &self.home
    }

    pub fn session_name(&self, chat_id: i64, mode: &str) -> String {
        format!("chatcli-tg-{}-{mode}", chat_id.unsigned_abs())
    }

    pub async fn active(&self, chat_id: i64) -> Option<TerminalStatus> {
        self.sessions
            .lock()
            .await
            .get(&chat_id)
            .map(|session| TerminalStatus {
                runner: Arc::clone(&session.runner),
                agent: session.agent,
            })
    }

    pub async fn replace(
        &self,
        chat_id: i64,
        runner: Arc<CliRunner>,
        agent: AgentKind,
        stop: tokio::sync::oneshot::Sender<()>,
        watcher: JoinHandle<()>,
    ) {
        self.close(chat_id).await;
        self.sessions.lock().await.insert(
            chat_id,
            TerminalSession {
                runner,
                agent,
                stop: Some(stop),
                watcher,
            },
        );
    }

    pub async fn close(&self, chat_id: i64) -> bool {
        let session = self.sessions.lock().await.remove(&chat_id);
        let Some(mut session) = session else {
            return false;
        };
        if let Some(stop) = session.stop.take() {
            let _ = stop.send(());
        }
        session.watcher.abort();
        let _ = session.runner.kill().await;
        true
    }
}

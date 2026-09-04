//! Agent Session 的纯领域模型。
//!
//! 本模块不读取文件、不持有锁，也不依赖 app、Telegram 或 tmux。

use crate::domain::AgentKind;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionState {
    Initial,
    CliSelected {
        kind: AgentKind,
    },
    WaitingWorkspace {
        kind: Option<AgentKind>,
        candidates: Vec<PathBuf>,
        #[serde(default)]
        workspace: Option<PathBuf>,
    },
    WaitingShellCommand {
        terminal_session_id: String,
        command: String,
        description: String,
    },
    ActiveTerminal {
        terminal_session_id: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub chat_id: i64,
    pub state: SessionState,
    pub status: SessionStatus,
    pub created: u64,
    pub updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEvent {
    pub session_id: String,
    pub chat_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingLaunch {
    pub kind: Option<AgentKind>,
    pub candidates: Vec<PathBuf>,
    #[serde(default)]
    pub workspace: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingShellCommand {
    pub terminal_session_id: String,
    pub command: String,
    pub description: String,
}

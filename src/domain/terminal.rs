use serde::{Deserialize, Serialize};

use super::AgentKind;

/// 一个 tmux 终端中运行的模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerminalKind {
    Shell,
    Codex,
    Cursor,
}

impl TerminalKind {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Shell => "终端",
            Self::Codex => "CodeX",
            Self::Cursor => "Cursor",
        }
    }
}

impl From<AgentKind> for TerminalKind {
    fn from(kind: AgentKind) -> Self {
        match kind {
            AgentKind::Codex => Self::Codex,
            AgentKind::Cursor => Self::Cursor,
        }
    }
}

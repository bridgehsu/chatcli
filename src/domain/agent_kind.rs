use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// 支持的 CLI Agent 类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentKind {
    Cursor,
    Codex,
}

impl AgentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cursor => "cursor",
            Self::Codex => "codex",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Cursor => "Cursor",
            Self::Codex => "CodeX",
        }
    }
}

impl fmt::Display for AgentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AgentKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "cursor" => Ok(AgentKind::Cursor),
            "codex" => Ok(AgentKind::Codex),
            _ => Err(()),
        }
    }
}

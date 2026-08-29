use std::fmt;
use std::str::FromStr;

/// 支持的 CLI Agent 类型（chatcli 统一入口）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentKind {
    Cursor,
    Claude,
    Codex,
}

impl AgentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AgentKind::Cursor => "cursor",
            AgentKind::Claude => "claude",
            AgentKind::Codex => "codex",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            AgentKind::Cursor => "Cursor",
            AgentKind::Claude => "Claude",
            AgentKind::Codex => "Codex",
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
            "claude" => Ok(AgentKind::Claude),
            "codex" => Ok(AgentKind::Codex),
            _ => Err(()),
        }
    }
}

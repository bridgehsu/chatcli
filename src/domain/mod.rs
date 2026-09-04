//! 核心业务模型：描述会话、终端和 Agent 类型。

pub mod agent_kind;
pub mod session;
pub mod terminal;

pub use agent_kind::AgentKind;
pub use session::{
    ContextEvent, PendingLaunch, PendingShellCommand, Session, SessionState as AgentSessionState,
    SessionStatus,
};
pub use terminal::TerminalKind;

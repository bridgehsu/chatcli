pub mod manager;
pub mod session_manager;
mod terminal_manager;

pub use manager::{SessionRecord, SessionState, TerminalManager};
pub use session_manager::{Session, SessionManager, SessionState as AgentSessionState};

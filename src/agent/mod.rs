/// CLI kinds supported by the interactive terminal bridge.
pub(crate) mod actions;
pub mod agent;
mod filesystem;
pub mod intent;
pub mod kind;
mod resolver;
mod terminal;

pub use agent::Agent;
pub(crate) use filesystem::{SearchResult, WorkspaceSearchTool};
pub use kind::AgentKind;
pub use resolver::{Decision, Resolver};
pub(crate) use terminal::TerminalTool;

/// CLI kinds supported by the interactive terminal bridge.
pub(crate) mod actions;
pub mod agent;
pub mod intent;
pub mod kind;
mod resolver;

pub use agent::Agent;
pub use kind::AgentKind;
pub use resolver::{Decision, Resolver};

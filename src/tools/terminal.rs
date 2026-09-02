use anyhow::Result;

use crate::{agent::AgentKind, infrastructure::CliRunner};

/// Controlled terminal capabilities exposed to the session layer.
#[derive(Default)]
pub struct TerminalTool;

impl TerminalTool {
    pub async fn start(&self, terminal: &CliRunner, agent: AgentKind) -> Result<()> {
        terminal.start_agent(agent).await
    }

    pub async fn send_input(&self, terminal: &CliRunner, input: &str) -> Result<()> {
        terminal.send_line(input).await
    }

    pub async fn send_keys(&self, terminal: &CliRunner, keys: &str) -> Result<()> {
        terminal.send_raw_keys(keys).await
    }

    pub async fn capture_screen(&self, terminal: &CliRunner) -> Result<String> {
        terminal.capture_pane_public().await
    }

    pub async fn attach_local(&self, terminal: &CliRunner) -> Result<()> {
        terminal.open_local_terminal().await
    }
}

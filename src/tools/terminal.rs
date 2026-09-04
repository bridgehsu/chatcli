use anyhow::Result;

use crate::{domain::AgentKind, infrastructure::CliRunner};

/// Agent 可调用的终端原生工具，不管理 Terminal Session 的归属或持久化。
#[derive(Default)]
pub struct TerminalTool;

impl TerminalTool {
    /// 创建或复用一个只运行默认 Shell 的 tmux 终端。
    pub async fn start_shell(&self, terminal: &CliRunner) -> Result<()> {
        terminal.start_shell().await
    }

    /// 在已准备好的 tmux 终端中启动指定 CLI。
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

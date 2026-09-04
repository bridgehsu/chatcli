use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::info;

use crate::domain::AgentKind;

const WINDOW: &str = "agent";

/// One real, persistent interactive terminal. The terminal belongs to one chat.
#[derive(Clone)]
pub struct CliRunner {
    pub workspace: PathBuf,
    pub tmux_session: String,
}

impl CliRunner {
    pub fn new(workspace: impl Into<PathBuf>, tmux_session: String) -> Self {
        Self {
            workspace: workspace.into(),
            tmux_session,
        }
    }

    pub fn target(&self) -> String {
        format!("{}:{}", self.tmux_session, WINDOW)
    }

    /// Creates a fresh tmux window backed by the user's default shell.
    /// This is an implementation detail: ChatCLI exposes only supported CLI modes.
    async fn prepare_terminal(&self) -> Result<()> {
        self.ensure_tmux_session().await?;
        self.prepare_window().await?;
        Ok(())
    }

    /// Start an AI CLI inside the existing real terminal manager.
    pub async fn start_agent(&self, kind: AgentKind) -> Result<()> {
        self.prepare_terminal().await?;
        let command = match kind {
            AgentKind::Codex => "exec codex",
            AgentKind::Cursor => "exec cursor agent",
        };
        self.send_line(command).await?;
        info!(session = %self.tmux_session, agent = %kind, "Started interactive CLI");
        Ok(())
    }

    /// 创建一个名称固定为 `agent` 的普通 Shell 窗口。
    ///
    /// Shell、CodeX、Cursor 共用同一个窗口命名约定，后续 send-keys、capture-pane
    /// 和 Terminal.app attach 才能稳定定位到同一目标。
    pub async fn start_shell(&self) -> Result<()> {
        self.prepare_terminal().await
    }

    pub async fn ensure_tmux_session(&self) -> Result<()> {
        let exists = Command::new("tmux")
            .args(["has-session", "-t", &self.tmux_session])
            .stderr(Stdio::null())
            .status()
            .await
            .map(|status| status.success())
            .unwrap_or(false);
        if !exists {
            Command::new("tmux")
                .args([
                    "new-session",
                    "-d",
                    "-s",
                    &self.tmux_session,
                    "-c",
                    self.workspace.to_str().unwrap_or("."),
                ])
                .status()
                .await
                .context("创建 tmux 会话失败；请确认 tmux 已安装")?;
        }
        Ok(())
    }

    pub async fn exists(&self) -> bool {
        Command::new("tmux")
            .args(["has-session", "-t", &self.tmux_session])
            .stderr(Stdio::null())
            .status()
            .await
            .map(|status| status.success())
            .unwrap_or(false)
    }

    async fn prepare_window(&self) -> Result<()> {
        let windows = Command::new("tmux")
            .args([
                "list-windows",
                "-t",
                &self.tmux_session,
                "-F",
                "#{window_name}",
            ])
            .output()
            .await
            .context("tmux list-windows failed")?;
        let exists = String::from_utf8_lossy(&windows.stdout)
            .lines()
            .any(|name| name.trim() == WINDOW);
        if exists {
            Command::new("tmux")
                .args(["kill-window", "-t", &self.target()])
                .status()
                .await?;
        }
        Command::new("tmux")
            .args([
                "new-window",
                "-d",
                "-t",
                &self.tmux_session,
                "-n",
                WINDOW,
                "-c",
                self.workspace.to_str().unwrap_or("."),
            ])
            .status()
            .await
            .context("创建 tmux 窗口失败")?;
        Ok(())
    }

    pub async fn send_line(&self, input: &str) -> Result<()> {
        let target = self.target();
        let status = Command::new("tmux")
            .args(["send-keys", "-t", &target, "-l", input])
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("无法向 tmux 窗口 {target} 发送输入");
        }
        let status = Command::new("tmux")
            .args(["send-keys", "-t", &target, "Enter"])
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("无法向 tmux 窗口 {target} 发送回车");
        }
        Ok(())
    }

    pub async fn send_raw_keys(&self, keys: &str) -> Result<()> {
        let status = Command::new("tmux")
            .args(["send-keys", "-t", &self.target(), keys])
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("无法向 tmux 窗口发送按键");
        }
        Ok(())
    }

    pub async fn capture_pane_public(&self) -> Result<String> {
        let output = Command::new("tmux")
            .args(["capture-pane", "-t", &self.target(), "-p", "-S", "-200"])
            .output()
            .await
            .context("tmux capture-pane failed")?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn kill(&self) -> Result<()> {
        Command::new("tmux")
            .args(["kill-session", "-t", &self.tmux_session])
            .status()
            .await
            .context("关闭 tmux 会话失败")?;
        Ok(())
    }

    pub async fn open_local_terminal(&self) -> Result<()> {
        let script = format!(
            "tell application \"Terminal\"\nactivate\nif (count of windows) is 0 then\n    do script \"tmux attach -t {}:agent\"\nelse\n    do script \"tmux attach -t {}:agent\" in front window\nend if\nend tell",
            self.tmux_session,
            self.tmux_session
        );
        Command::new("osascript")
            .arg("-e")
            .arg(script)
            .status()
            .await
            .context("无法打开 macOS Terminal")?;
        Ok(())
    }
}

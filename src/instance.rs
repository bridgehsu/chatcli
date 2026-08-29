use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};
use tracing::info;

/// Hold the PID lock file for the process lifetime.
pub struct InstanceGuard {
    path: PathBuf,
}

impl InstanceGuard {
    pub fn acquire() -> Result<Self> {
        let path = dirs::home_dir()
            .context("无法确定 home 目录")?
            .join(".chatcli")
            .join("chatcli.pid");

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if path.exists() {
            if let Ok(raw) = std::fs::read_to_string(&path) {
                if let Ok(pid) = raw.trim().parse::<u32>() {
                    if pid != std::process::id() && is_process_alive(pid) {
                        bail!(
                            "已有 chatcli 在运行 (PID {})。\n\
                             请先执行 VS Code 任务「停止 chatcli」，或运行：\n\
                             pkill -f 'target/debug/chatcli'",
                            pid
                        );
                    }
                }
            }
        }

        std::fs::write(&path, std::process::id().to_string())?;
        info!(pid = std::process::id(), path = %path.display(), "Acquired instance lock");

        Ok(Self { path })
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            if let Ok(raw) = std::fs::read_to_string(&self.path) {
                if raw.trim() == std::process::id().to_string() {
                    let _ = std::fs::remove_file(&self.path);
                }
            }
        }
    }
}

fn is_process_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

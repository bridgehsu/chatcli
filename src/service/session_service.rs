//! Agent Session 的操作：创建、切换、列表与调试重置。

use crate::{
    app::Agent,
    domain::{AgentSessionState as SessionState, Session},
    service::output::ChatOutput,
};
use std::sync::Arc;
use teloxide::types::ChatId;

pub async fn list(output: &dyn ChatOutput, chat: ChatId, state: &Arc<Agent>, switching: bool) {
    let (sessions, current) = state.list_sessions(chat).await;
    let _ = output
        .send(
            chat.0,
            session_list_text(&sessions, current.as_deref(), switching),
        )
        .await;
}

/// 从 Agent 会话列表中切换，并恢复其流程状态和当前终端。
pub async fn switch(output: &dyn ChatOutput, chat: ChatId, state: &Arc<Agent>, id: &str) {
    match state.switch_session(chat, id).await {
        Ok(session) => {
            let message = match session.state {
                SessionState::Initial => "已切换到初始会话。".to_owned(),
                SessionState::CliSelected { kind } => {
                    format!("已切换到 {} 会话中心。", kind.display_name())
                }
                SessionState::WaitingWorkspace { .. } => {
                    "已恢复目录选择流程，请继续回复目录序号或路径。".to_owned()
                }
                SessionState::ActiveTerminal { .. } => {
                    "已切换到包含活动终端的 Agent 会话。".to_owned()
                }
                SessionState::WaitingShellCommand { .. } => {
                    "已恢复待确认的 Shell 命令，请回复“确认”执行或回复“取消”放弃。".to_owned()
                }
            };
            let _ = output.send(chat.0, message).await;
        }
        Err(error) => {
            let _ = output.send(chat.0, error).await;
        }
    }
}

/// 关闭当前 Agent Session；后台 tmux 终端不会被杀掉，但不再属于当前聊天流程。
pub async fn close(output: &dyn ChatOutput, chat: ChatId, state: &Arc<Agent>) {
    match state.close_session(chat).await {
        Some(session) => {
            let _ = output
                .send(
                    chat.0,
                    format!("已关闭 Agent 会话：{}。已创建新的初始会话。", session.id),
                )
                .await;
        }
        None => {
            let _ = output
                .send(chat.0, "当前没有可关闭的 Agent 会话。".to_owned())
                .await;
        }
    }
}

/// 创建新的 Agent Session；旧会话及其 tmux 终端保留，可稍后重新切换。
pub(crate) async fn new_session(output: &dyn ChatOutput, chat: ChatId, state: &Arc<Agent>) {
    state.clear_pending_workspace(chat).await;
    let _ = state.take_pending_shell_command(chat).await;
    state.clear_selected_cli(chat).await;
    state.new_session(chat).await;
    state
        .record_context(chat, "system", "创建新的 Agent Session")
        .await;

    let _ = output
        .send(
            chat.0,
            "已开启新的会话。你可以发送：打开终端、CodeX 或 Cursor。".to_owned(),
        )
        .await;
}

/// 渠道无关的会话列表文本；具体渠道可自行做富文本渲染。
fn session_list_text(sessions: &[Session], current: Option<&str>, switching: bool) -> String {
    if sessions.is_empty() {
        return "Agent 会话\n\n当前没有可用会话。".to_owned();
    }
    let mut lines = vec![format!("Agent 会话（共 {} 个）", sessions.len())];
    for session in sessions {
        let marker = if current == Some(session.id.as_str()) {
            "● 当前"
        } else {
            "○"
        };
        lines.push(format!(
            "{} · {}\nID: {}",
            marker,
            state_label(&session.state),
            session.id
        ));
    }
    if switching {
        lines.push("切换方式：/agent_session 会话ID".to_owned());
    }
    lines.join("\n\n")
}

fn state_label(state: &SessionState) -> String {
    match state {
        SessionState::Initial => "初始化".to_owned(),
        SessionState::CliSelected { kind } => format!("已选择 {}", kind.display_name()),
        SessionState::WaitingWorkspace { .. } => "等待选择目录".to_owned(),
        SessionState::WaitingShellCommand { .. } => "等待确认 Shell 命令".to_owned(),
        SessionState::ActiveTerminal { .. } => "正在使用终端".to_owned(),
    }
}

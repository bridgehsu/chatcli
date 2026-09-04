//! 与具体 Session、Terminal、Workspace 无关的系统级反馈。

use crate::{app::Agent, domain::AgentSessionState, service::output::ChatOutput};
use std::sync::Arc;
use teloxide::types::ChatId;
use tracing::info;

/// 根据当前流程状态展示帮助；不会改变任何 Session 状态。
pub async fn help(output: &dyn ChatOutput, chat: ChatId, state: &Arc<Agent>) {
    let current = state.get_current_session(chat).await;
    let message = match current.state {
        Some(AgentSessionState::WaitingWorkspace { .. }) => {
            "当前正在选择工作目录。请回复目录序号或路径；发送“取消”退出。"
        }
        Some(AgentSessionState::WaitingShellCommand { .. }) => {
            "当前有一条 Shell 命令等待确认。回复“确认”执行，或回复“取消”放弃。"
        }
        _ => "可发送：打开终端、CodeX、Cursor、新建会话、会话列表、终端列表、切换终端或 /reset。",
    };
    let _ = output.send(chat.0, message.to_owned()).await;
}

/// 说明 ChatCLI 的身份与当前支持的主要能力。
pub async fn introduce(output: &dyn ChatOutput, chat: ChatId, state: &Arc<Agent>) {
    let message = "我是 ChatCLI：通过 Telegram 管理本机 Shell、CodeX 和 Cursor 的 Agent。\n\n我可以创建和切换 Agent 会话、选择工作目录、启动或连接 tmux 终端，并将普通消息转发到当前终端。\n\n你可以直接发送：打开终端、CodeX、Cursor、新建会话、会话列表、终端列表、切换终端、获取当前目录下的目录、/help 或 /reset。";
    state.record_context(chat, "agent", message).await;
    let _ = output.send(chat.0, message.to_owned()).await;
}

/// 调试系统命令：清空当前 chat 的所有本地 Agent 数据并回到全新初始会话。
pub async fn reset(output: &dyn ChatOutput, chat: ChatId, state: &Arc<Agent>) {
    info!(chat_id = chat.0, "Resetting all Agent data for chat");
    state.reset_chat(chat).await;
    match output
        .send(
            chat.0,
            "已清空当前聊天的会话、上下文和终端信息，并创建新的初始会话。".to_owned(),
        )
        .await
    {
        Ok(_) => info!(chat_id = chat.0, "Rendered Telegram reset response"),
        Err(error) => tracing::error!(chat_id = chat.0, %error, "Failed to send reset response"),
    }
}

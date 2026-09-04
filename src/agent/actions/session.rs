use crate::{
    agent::{actions::cli, Agent, AgentKind},
    channels::telegram::presenter,
    manager::AgentSessionState as SessionState,
};
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{ChatId, ParseMode},
};
use tracing::{error, info};

pub async fn list(bot: &Bot, chat: ChatId, state: &Arc<Agent>, kind: AgentKind) {
    let records = state.terminals.list(chat.0, kind).await;
    let current = state.terminals.active(chat.0).await.map(|s| s.id);
    let _ = bot
        .send_message(
            chat,
            presenter::session_list_text(&records, current.as_deref(), kind),
        )
        .reply_markup(presenter::list_keyboard(&records))
        .await;
}

pub async fn list_sessions(bot: &Bot, chat: ChatId, state: &Arc<Agent>, switching: bool) {
    let (sessions, current) = state.list_sessions(chat).await;
    let text = if sessions.is_empty() {
        "当前没有 Agent 会话。".to_owned()
    } else {
        let rows = sessions
            .iter()
            .map(|session| {
                let marker = if current.as_deref() == Some(session.id.as_str()) {
                    "● 当前"
                } else {
                    "○"
                };
                format!(
                    "{marker} · {}\n状态：{:?}\nID: {}",
                    session.id, session.state, session.id
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let suffix = if switching {
            "\n\n发送 /agent_session 会话ID 可切换到对应会话。"
        } else {
            ""
        };
        format!("Agent 会话列表\n\n{rows}{suffix}")
    };
    let mut request = bot.send_message(chat, text);
    if switching {
        request = request.reply_markup(presenter::agent_session_keyboard(&sessions));
    }
    let _ = request.await;
}

/// 从“Agent 会话列表”点击后恢复所选会话的流程状态与关联终端。
pub async fn switch_agent_session(bot: &Bot, chat: ChatId, state: &Arc<Agent>, id: &str) {
    match state.switch_session(chat, id).await {
        Ok(session) => {
            let message = match session.state {
                SessionState::Initial => "已切换到初始会话，请选择 CodeX 或 Cursor。".to_owned(),
                SessionState::CliSelected { kind } => {
                    format!("已切换到 {} 会话中心。", kind.display_name())
                }
                SessionState::WaitingWorkspace { .. } => {
                    "已恢复目录选择流程，请继续回复目录序号或路径。".to_owned()
                }
                SessionState::ActiveTerminal { kind, .. } => format!(
                    "已切换到 {} 终端会话，后续普通消息将转发到该 CLI。",
                    kind.display_name()
                ),
            };
            let _ = bot.send_message(chat, message).await;
        }
        Err(error) => {
            let _ = bot.send_message(chat, error).await;
        }
    }
}

pub async fn select(bot: &Bot, chat: ChatId, state: &Arc<Agent>, id: &str) {
    if state.terminals.select(chat.0, id).await {
        let active = state
            .terminals
            .active(chat.0)
            .await
            .expect("selected manager exists");
        state.selected.lock().await.insert(chat.0, active.agent);
        state.persist_agent_session(chat).await;
        let _ = bot
            .send_message(
                chat,
                format!("已切换到当前 {} 会话。", active.agent.display_name()),
            )
            .reply_markup(presenter::active_keyboard(active.agent))
            .await;
    } else {
        let _ = bot
            .send_message(chat, "该会话不可用：它可能已结束或 tmux 已失效。")
            .await;
    }
}
pub async fn reset(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    info!(chat_id = chat.0, "Resetting current manager state");
    state.pending.lock().await.remove(&chat.0);
    state.selected.lock().await.remove(&chat.0);
    state.terminals.clear_current(chat.0).await;
    state.persist_agent_session(chat).await;
    info!(chat_id = chat.0, "Session state reset and persisted");
    match bot
        .send_message(chat, "已重置机器人调试状态。后台会话仍在运行。")
        .reply_markup(presenter::initial_keyboard())
        .await
    {
        Ok(_) => info!(chat_id = chat.0, "Rendered Telegram reset keyboard"),
        Err(error) => error!(chat_id = chat.0, %error, "Failed to render Telegram reset keyboard"),
    }
}
pub async fn forward(bot: &Bot, chat: ChatId, state: &Arc<Agent>, text: String) {
    let Some(active) = state.terminals.active(chat.0).await else {
        return;
    };
    info!(
        chat_id = chat.0,
        terminal_id = %active.id,
        agent = %active.agent.display_name(),
        text_len = text.chars().count(),
        "Forwarding message to active terminal"
    );
    if let Err(error) = state.terminal.send_input(&active.runner, &text).await {
        error!(chat_id = chat.0, terminal_id = %active.id, %error, "Failed to forward message to active terminal");
        let _ = bot.send_message(chat, format!("发送失败：{error}")).await;
    } else {
        info!(chat_id = chat.0, terminal_id = %active.id, "Forwarded message to active terminal");
    }
}
pub async fn stop(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    match state.terminals.active(chat.0).await {
        Some(active) => {
            let _ = state.terminal.send_keys(&active.runner, "C-c").await;
            let _ = bot.send_message(chat, "已发送 Ctrl+C。").await;
        }
        None => {
            let _ = bot.send_message(chat, "当前没有选中的会话。").await;
        }
    }
}
pub async fn close(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    if state.terminals.close_current(chat.0).await {
        let selected = { state.selected.lock().await.get(&chat.0).copied() };
        if let Some(kind) = selected {
            cli::home(bot, chat, state, kind).await
        } else {
            cli::initial(bot, chat, state, "当前会话已结束。").await
        }
    } else {
        let _ = bot.send_message(chat, "当前没有选中的会话。").await;
    }
}
pub async fn attach(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    let Some(active) = state.terminals.active(chat.0).await else {
        let _ = bot.send_message(chat, "当前没有选中的会话。").await;
        return;
    };
    let message = match state.terminal.attach_local(&active.runner).await {
        Ok(()) => "已在 Mac Terminal 中接入当前 tmux 会话。".into(),
        Err(error) => format!("打开本机终端失败：{error}"),
    };
    let _ = bot.send_message(chat, message).await;
}
pub async fn status(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    let text = match state.terminals.active(chat.0).await {
        Some(active) => format!(
            "当前会话 · {}\n目录：{}",
            active.agent.display_name(),
            active.runner.workspace.display()
        ),
        None => "当前没有选中的会话。".into(),
    };
    let _ = bot.send_message(chat, text).await;
}
pub async fn screen(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    let Some(active) = state.terminals.active(chat.0).await else {
        let _ = bot.send_message(chat, "当前没有选中的会话。").await;
        return;
    };
    match state.terminal.capture_screen(&active.runner).await {
        Ok(view) => {
            let _ = bot
                .send_message(
                    chat,
                    format!(
                        "<pre>{}</pre>",
                        presenter::html_escape(&presenter::tail_chars(
                            &presenter::strip_ansi(&view),
                            3500
                        ))
                    ),
                )
                .parse_mode(ParseMode::Html)
                .await;
        }
        Err(error) => {
            let _ = bot
                .send_message(chat, format!("读取终端失败：{error}"))
                .await;
        }
    }
}

/// 开启新的 Agent 会话上下文，不关闭已经在后台运行的 tmux CLI 会话。
pub(crate) async fn new_session(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    // 新会话从 Initial 状态开始：不再沿用旧流程的 CLI 选择、目录候选或当前终端。
    state.pending.lock().await.remove(&chat.0);
    state.selected.lock().await.remove(&chat.0);
    state.terminals.clear_current(chat.0).await;
    state.new_session(chat).await;
    state
        .record_context(chat, "system", "创建新的 Agent Session")
        .await;

    let _ = bot
        .send_message(
            chat,
            "已开启新的会话，请选择要使用的 CLI：Codex 或 Cursor。",
        )
        .reply_markup(presenter::initial_keyboard())
        .await;
}

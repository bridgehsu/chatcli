use crate::{
    agent::{actions::cli, Agent, AgentKind},
    channels::telegram::presenter,
};
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{ChatId, ParseMode},
};

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
pub async fn select(bot: &Bot, chat: ChatId, state: &Arc<Agent>, id: &str) {
    if state.terminals.select(chat.0, id).await {
        let active = state
            .terminals
            .active(chat.0)
            .await
            .expect("selected session exists");
        state.selected.lock().await.insert(chat.0, active.agent);
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
    state.pending.lock().await.remove(&chat.0);
    state.selected.lock().await.remove(&chat.0);
    state.terminals.clear_current(chat.0).await;
    let _ = bot
        .send_message(chat, "已重置机器人调试状态。后台会话仍在运行。")
        .reply_markup(presenter::initial_keyboard())
        .await;
}
pub async fn forward(bot: &Bot, chat: ChatId, state: &Arc<Agent>, text: String) {
    let Some(active) = state.terminals.active(chat.0).await else {
        return;
    };
    if let Err(error) = state.terminal.send_input(&active.runner, &text).await {
        let _ = bot.send_message(chat, format!("发送失败：{error}")).await;
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
        if let Some(kind) = state.selected.lock().await.get(&chat.0).copied() {
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

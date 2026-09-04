//! Terminal Session 的操作：创建、切换、转发、查看和关闭 tmux 终端。

use crate::{
    agent::TerminalKind, app::Agent, infrastructure::CliRunner,
    interfaces::channels::telegram::presenter,
};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use teloxide::{
    prelude::*,
    types::{ChatId, ParseMode},
};
use tracing::{error, info};

/// 展示当前 Agent Session 下的所有 Terminal Session。
pub async fn list(bot: &Bot, chat: ChatId, state: &Arc<Agent>, switching: bool) {
    let agent_session_id = state.current_session_id(chat).await;
    let records = state.terminals.list(&agent_session_id).await;
    let current = state
        .terminals
        .active(&agent_session_id)
        .await
        .map(|terminal| terminal.id);
    let mut request = bot.send_message(
        chat,
        presenter::session_list_text(&records, current.as_deref()),
    );
    if switching {
        request = request.reply_markup(presenter::list_keyboard(&records));
    }
    let _ = request.await;
}

/// 在当前 Agent Session 中创建普通 Shell，并在 macOS Terminal.app 接入对应 tmux 会话。
pub async fn open_shell(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    let agent_session_id = state.current_session_id(chat).await;
    let id = format!(
        "shell-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let workspace = state.terminals.home().clone();
    let runner = Arc::new(CliRunner::new(
        workspace.clone(),
        state.terminals.session_name(chat.0, &id),
    ));
    if let Err(error) = state.terminal.start_shell(&runner).await {
        let _ = bot
            .send_message(chat, format!("创建终端失败：{error}"))
            .await;
        return;
    }

    let (stop, watcher) = crate::interfaces::channels::telegram::terminal_view::spawn(
        bot.clone(),
        chat,
        Arc::clone(&runner),
    );
    state
        .terminals
        .create(
            chat.0,
            agent_session_id,
            id,
            Arc::clone(&runner),
            TerminalKind::Shell,
            stop,
            watcher,
        )
        .await;
    state.persist_agent_session(chat).await;

    let message = match state.terminal.attach_local(&runner).await {
        Ok(()) => format!(
            "已打开 macOS Terminal，并接入普通 tmux 终端。\n目录：{}",
            workspace.display()
        ),
        Err(error) => format!("终端已创建，但打开 macOS Terminal 失败：{error}"),
    };
    let _ = bot.send_message(chat, message).await;
}

/// 将当前 Agent Session 的输入目标切换到指定 Terminal Session。
pub async fn switch(bot: &Bot, chat: ChatId, state: &Arc<Agent>, id: &str) {
    let agent_session_id = state.current_session_id(chat).await;
    if state.terminals.select(&agent_session_id, id).await {
        let active = state
            .terminals
            .active(&agent_session_id)
            .await
            .expect("selected terminal must be active");
        state.persist_agent_session(chat).await;
        let _ = bot
            .send_message(
                chat,
                format!("已切换到当前 {} 终端。", active.kind.display_name()),
            )
            .await;
    } else {
        let _ = bot
            .send_message(chat, "该终端不可用：它可能已结束或 tmux 已失效。")
            .await;
    }
}

/// 将 Telegram 的普通文本输入到当前 Terminal Session。
pub async fn forward(bot: &Bot, chat: ChatId, state: &Arc<Agent>, text: String) {
    let agent_session_id = state.current_session_id(chat).await;
    let Some(active) = state.terminals.active(&agent_session_id).await else {
        return;
    };
    info!(
        chat_id = chat.0,
        terminal_id = %active.id,
        terminal = %active.kind.display_name(),
        text_len = text.chars().count(),
        "Forwarding message to active terminal"
    );
    let mut result = state.terminal.send_input(&active.runner, &text).await;
    // 兼容旧版 Shell：旧实现只创建 tmux 默认窗口，没有创建 `agent` 窗口。
    // 第一次发送失败时补建 Shell 窗口并重试一次，避免用户必须手动删除旧会话。
    if result.is_err() && active.kind == TerminalKind::Shell {
        if let Err(error) = state.terminal.start_shell(&active.runner).await {
            result = Err(error);
        } else {
            result = state.terminal.send_input(&active.runner, &text).await;
        }
    }
    if let Err(error) = result {
        error!(chat_id = chat.0, terminal_id = %active.id, %error, "Failed to forward message to active terminal");
        let _ = bot.send_message(chat, format!("发送失败：{error}")).await;
    } else {
        info!(chat_id = chat.0, terminal_id = %active.id, "Forwarded message to active terminal");
    }
}

pub async fn close(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    let agent_session_id = state.current_session_id(chat).await;
    if state.terminals.close_current(&agent_session_id).await {
        state.persist_agent_session(chat).await;
        let _ = bot.send_message(chat, "当前终端已结束。").await;
    } else {
        let _ = bot.send_message(chat, "当前没有选中的终端。").await;
    }
}

pub async fn attach(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    let agent_session_id = state.current_session_id(chat).await;
    let Some(active) = state.terminals.active(&agent_session_id).await else {
        let _ = bot.send_message(chat, "当前没有选中的终端。").await;
        return;
    };
    let message = match state.terminal.attach_local(&active.runner).await {
        Ok(()) => "已在 Mac Terminal 中接入当前 tmux 终端。".into(),
        Err(error) => format!("打开本机终端失败：{error}"),
    };
    let _ = bot.send_message(chat, message).await;
}

pub async fn screen(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    let agent_session_id = state.current_session_id(chat).await;
    let Some(active) = state.terminals.active(&agent_session_id).await else {
        let _ = bot.send_message(chat, "当前没有选中的终端。").await;
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

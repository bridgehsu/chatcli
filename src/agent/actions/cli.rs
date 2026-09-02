use crate::agent::AgentKind;
use crate::{
    agent::Agent,
    channels::telegram::{presenter, terminal_view},
    infrastructure::CliRunner,
};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use teloxide::{
    prelude::*,
    types::{ChatAction, ChatId, ParseMode},
};

pub async fn initial(bot: &Bot, chat: ChatId, state: &Arc<Agent>, message: &str) {
    state.selected.lock().await.remove(&chat.0);
    let _ = bot
        .send_message(chat, message)
        .reply_markup(presenter::initial_keyboard())
        .await;
}

pub async fn home(bot: &Bot, chat: ChatId, state: &Arc<Agent>, kind: AgentKind) {
    state.selected.lock().await.insert(chat.0, kind);
    let _ = bot
        .send_message(chat, format!("{} 会话中心", kind.display_name()))
        .reply_markup(presenter::cli_home_keyboard(kind))
        .await;
}

pub async fn begin(bot: &Bot, chat: ChatId, state: &Arc<Agent>, kind: AgentKind) {
    state.selected.lock().await.insert(chat.0, kind);
    state.pending.lock().await.insert(
        chat.0,
        crate::agent::agent::PendingLaunch {
            kind,
            candidates: vec![],
        },
    );
    let _ = bot
        .send_message(
            chat,
            format!("请发送要启动 {} 的工作目录。", kind.display_name()),
        )
        .reply_markup(presenter::create_keyboard())
        .await;
}

pub async fn open(bot: &Bot, chat: ChatId, state: Arc<Agent>, kind: AgentKind, workspace: PathBuf) {
    let id = format!(
        "{}-{}",
        kind.as_str(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let runner = Arc::new(CliRunner::new(
        workspace.clone(),
        state.terminals.session_name(chat.0, &id),
    ));
    let _ = bot.send_chat_action(chat, ChatAction::Typing).await;
    if let Err(error) = state.terminal.start(&runner, kind).await {
        let _ = bot
            .send_message(chat, format!("启动 {} 失败：{error}", kind.display_name()))
            .reply_markup(presenter::cli_home_keyboard(kind))
            .await;
        return;
    }
    let (stop, watcher) = terminal_view::spawn(bot.clone(), chat, Arc::clone(&runner));
    state
        .terminals
        .create(chat.0, id, runner, kind, stop, watcher)
        .await;
    let _ = bot.send_message(chat, format!("已启动 {} 会话，目录：<code>{}</code>。\n它已设为当前会话；现在发送的普通消息会输入该 CLI。", kind.display_name(), presenter::html_escape(&workspace.display().to_string()))).parse_mode(ParseMode::Html).reply_markup(presenter::active_keyboard(kind)).await;
}

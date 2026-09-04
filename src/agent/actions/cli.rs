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
use tracing::{error, info};

pub async fn initial(bot: &Bot, chat: ChatId, state: &Arc<Agent>, message: &str) {
    state.selected.lock().await.remove(&chat.0);
    state.persist_agent_session(chat).await;
    match bot
        .send_message(chat, message)
        .reply_markup(presenter::initial_keyboard())
        .await
    {
        Ok(_) => info!(chat_id = chat.0, "Rendered Telegram initial keyboard"),
        Err(error) => {
            error!(chat_id = chat.0, %error, "Failed to render Telegram initial keyboard")
        }
    }
}

pub async fn home(bot: &Bot, chat: ChatId, state: &Arc<Agent>, kind: AgentKind) {
    state.selected.lock().await.insert(chat.0, kind);
    state.persist_agent_session(chat).await;
    let _ = bot
        .send_message(chat, format!("{} 会话中心", kind.display_name()))
        .reply_markup(presenter::cli_home_keyboard(kind))
        .await;
}

pub async fn begin(bot: &Bot, chat: ChatId, state: &Arc<Agent>, kind: AgentKind) {
    state.selected.lock().await.insert(chat.0, kind);
    // 用户可能先从目录列表选择了工作目录，再选择 CLI；这时直接启动即可。
    let selected_workspace = {
        state
            .pending
            .lock()
            .await
            .get(&chat.0)
            .and_then(|pending| pending.workspace.clone())
    };
    if let Some(workspace) = selected_workspace {
        state.pending.lock().await.remove(&chat.0);
        state.persist_agent_session(chat).await;
        open(bot, chat, Arc::clone(state), kind, workspace).await;
        return;
    }
    state.pending.lock().await.insert(
        chat.0,
        crate::agent::agent::PendingLaunch {
            kind: Some(kind),
            candidates: vec![],
            workspace: None,
        },
    );
    state.persist_agent_session(chat).await;
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
    state.persist_agent_session(chat).await;
    let _ = bot.send_message(chat, format!("已启动 {} 会话，目录：<code>{}</code>。\n它已设为当前会话；现在发送的普通消息会输入该 CLI。", kind.display_name(), presenter::html_escape(&workspace.display().to_string()))).parse_mode(ParseMode::Html).reply_markup(presenter::active_keyboard(kind)).await;
}

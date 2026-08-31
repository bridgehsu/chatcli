use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use teloxide::{
    prelude::*,
    types::{ChatAction, ParseMode},
};
use tokio::sync::Mutex;
use tracing::{info, warn};

use super::{
    commands::{self, Command},
    presenter, terminal_view,
};
use crate::{agent::AgentKind, cli::CliRunner, config::Config, terminal::TerminalManager};

struct AppState {
    terminals: TerminalManager,
    pending_launches: Mutex<HashMap<i64, PendingLaunch>>,
}

#[derive(Default)]
struct PendingLaunch {
    kind: Option<AgentKind>,
    prompt: Option<String>,
}

pub async fn start_bot(config: Arc<Config>) {
    let home = dirs::home_dir().expect("无法确定当前用户的 Home 目录");
    let state = Arc::new(AppState {
        terminals: TerminalManager::new(home),
        pending_launches: Mutex::new(HashMap::new()),
    });
    let allowed_ids = config.telegram.allowed_user_ids.clone();
    let bot = Bot::new(&config.telegram.token);
    info!(home = %state.terminals.home().display(), "ChatCLI terminal bridge starting");
    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let state = Arc::clone(&state);
        let allowed_ids = allowed_ids.clone();
        async move {
            handle_message(bot, msg, state, allowed_ids).await;
            Ok(())
        }
    })
    .await;
}

async fn handle_message(bot: Bot, msg: Message, state: Arc<AppState>, allowed: Vec<i64>) {
    let chat_id = msg.chat.id;
    let user_id = msg
        .from
        .as_ref()
        .map(|user| user.id.0 as i64)
        .unwrap_or_default();
    if !allowed.contains(&user_id) {
        warn!(user_id, "Rejected unauthorized Telegram message");
        return;
    }
    let Some(text) = msg.text().map(str::trim).filter(|text| !text.is_empty()) else {
        return;
    };
    match commands::parse(text) {
        Command::Help => {
            let _ = bot
                .send_message(chat_id, presenter::help_text())
                .reply_markup(presenter::main_keyboard())
                .await;
        }
        Command::Status => send_status(&bot, chat_id, &state).await,
        Command::Stop => interrupt(&bot, chat_id, &state).await,
        Command::Close => close_terminal(&bot, chat_id, &state).await,
        Command::Attach => attach_local(&bot, chat_id, &state).await,
        Command::Screen => send_screen(&bot, chat_id, &state).await,
        Command::OpenMenu => show_open_menu(&bot, chat_id, &state).await,
        Command::OpenAgent(kind, prompt) => {
            choose_workspace(&bot, chat_id, &state, kind, prompt).await
        }
        Command::ChooseAgent(kind) => {
            let prompt = state
                .pending_launches
                .lock()
                .await
                .remove(&chat_id.0)
                .and_then(|pending| pending.prompt);
            choose_workspace(&bot, chat_id, &state, kind, prompt).await;
        }
        Command::UseHome => launch_in_workspace(&bot, chat_id, Arc::clone(&state), "~").await,
        Command::Cancel => {
            state.pending_launches.lock().await.remove(&chat_id.0);
            let _ = bot.send_message(chat_id, "已取消。").await;
        }
        Command::Text(text) => handle_text(&bot, chat_id, Arc::clone(&state), text).await,
    }
}

async fn handle_text(bot: &Bot, chat_id: ChatId, state: Arc<AppState>, text: String) {
    if let Some(status) = state.terminals.active(chat_id.0).await {
        info!(chat_id = chat_id.0, "Forwarding chat input to terminal");
        if let Err(error) = status.runner.send_line(&text).await {
            let _ = bot
                .send_message(chat_id, format!("发送失败：{error}"))
                .await;
        }
    } else if state
        .pending_launches
        .lock()
        .await
        .get(&chat_id.0)
        .is_some_and(|pending| pending.kind.is_some())
    {
        launch_in_workspace(bot, chat_id, state, &text).await;
    } else {
        state.pending_launches.lock().await.insert(
            chat_id.0,
            PendingLaunch {
                kind: None,
                prompt: Some(text),
            },
        );
        let _ = bot
            .send_message(chat_id, "当前没有正在运行的 CLI。请选择要启动的 CLI。")
            .reply_markup(presenter::open_keyboard())
            .await;
    }
}

async fn choose_workspace(
    bot: &Bot,
    chat_id: ChatId,
    state: &AppState,
    kind: AgentKind,
    prompt: Option<String>,
) {
    if let Some(active) = state.terminals.active(chat_id.0).await {
        let _ = bot
            .send_message(
                chat_id,
                format!(
                    "当前正在运行 {}。请先用 /close 关闭后再启动另一个 CLI。",
                    active.agent.display_name()
                ),
            )
            .await;
        return;
    }
    state.pending_launches.lock().await.insert(
        chat_id.0,
        PendingLaunch {
            kind: Some(kind),
            prompt,
        },
    );
    let _ = bot
        .send_message(
            chat_id,
            format!(
                "{} 将在哪个目录中启动？\n发送绝对路径、相对 Home 的路径或 <code>~/项目目录</code>。",
                kind.display_name()
            ),
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(presenter::workspace_keyboard())
        .await;
}

async fn launch_in_workspace(bot: &Bot, chat_id: ChatId, state: Arc<AppState>, input: &str) {
    let pending = state.pending_launches.lock().await.remove(&chat_id.0);
    let Some(PendingLaunch {
        kind: Some(kind),
        prompt,
    }) = pending
    else {
        let _ = bot
            .send_message(chat_id, "请先用 /open 选择要启动的 CLI。")
            .await;
        return;
    };
    let workspace = match crate::terminal::workspace::resolve(state.terminals.home(), input) {
        Ok(workspace) => workspace,
        Err(error) => {
            state.pending_launches.lock().await.insert(
                chat_id.0,
                PendingLaunch {
                    kind: Some(kind),
                    prompt,
                },
            );
            let _ = bot
                .send_message(chat_id, format!("{error}\n请重新发送目录路径。"))
                .await;
            return;
        }
    };
    open_agent(bot, chat_id, state, kind, prompt, workspace).await;
}

async fn show_open_menu(bot: &Bot, chat_id: ChatId, state: &AppState) {
    if let Some(active) = state.terminals.active(chat_id.0).await {
        let _ = bot
            .send_message(
                chat_id,
                format!(
                    "当前正在运行 {}。请用 /screen、/attach、/stop 或 /close 操作该终端。",
                    active.agent.display_name()
                ),
            )
            .await;
        return;
    }
    let _ = bot
        .send_message(chat_id, "请选择要启动的 CLI。")
        .reply_markup(presenter::open_keyboard())
        .await;
}

async fn open_agent(
    bot: &Bot,
    chat_id: ChatId,
    state: Arc<AppState>,
    kind: AgentKind,
    prompt: Option<String>,
    workspace: PathBuf,
) {
    if let Some(active) = state.terminals.active(chat_id.0).await {
        let _ = bot
            .send_message(
                chat_id,
                format!(
                    "当前正在运行 {}。请先用 /close 关闭后再启动另一个 CLI。",
                    active.agent.display_name()
                ),
            )
            .await;
        return;
    }
    let runner = Arc::new(CliRunner::new(
        workspace.clone(),
        state.terminals.session_name(chat_id.0, kind.as_str()),
    ));
    let _ = bot.send_chat_action(chat_id, ChatAction::Typing).await;
    if let Err(error) = runner.start_agent(kind).await {
        let _ = bot
            .send_message(
                chat_id,
                format!("启动 {} 失败：{error}", kind.display_name()),
            )
            .await;
        return;
    }
    let (stop, watcher) = terminal_view::spawn(bot.clone(), chat_id, Arc::clone(&runner));
    state
        .terminals
        .replace(chat_id.0, Arc::clone(&runner), kind, stop, watcher)
        .await;
    let _ = bot.send_message(chat_id, format!("已在真实终端中启动 {}，目录：<code>{}</code>。\n后续聊天消息会输入到该 CLI；/attach 可在电脑接管。", kind.display_name(), presenter::html_escape(&workspace.display().to_string()))).parse_mode(ParseMode::Html).await;
    if let Some(prompt) = prompt {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let _ = runner.send_line(&prompt).await;
    }
}

async fn interrupt(bot: &Bot, chat_id: ChatId, state: &AppState) {
    match state.terminals.active(chat_id.0).await {
        Some(status) => {
            let _ = status.runner.send_raw_keys("C-c").await;
            let _ = bot.send_message(chat_id, "已发送 Ctrl+C。").await;
        }
        None => {
            let _ = bot.send_message(chat_id, "当前没有打开的终端。").await;
        }
    }
}
async fn close_terminal(bot: &Bot, chat_id: ChatId, state: &AppState) {
    let message = if state.terminals.close(chat_id.0).await {
        "终端已关闭。"
    } else {
        "当前没有打开的终端。"
    };
    let _ = bot.send_message(chat_id, message).await;
}
async fn attach_local(bot: &Bot, chat_id: ChatId, state: &AppState) {
    let Some(status) = state.terminals.active(chat_id.0).await else {
        let _ = bot.send_message(chat_id, "当前没有打开的终端。").await;
        return;
    };
    let message = match status.runner.open_local_terminal().await {
        Ok(()) => "已在 Mac Terminal 中接入同一个 tmux 终端。".into(),
        Err(error) => format!("打开本机终端失败：{error}"),
    };
    let _ = bot.send_message(chat_id, message).await;
}
async fn send_status(bot: &Bot, chat_id: ChatId, state: &AppState) {
    let message = match state.terminals.active(chat_id.0).await {
        Some(status) => format!(
            "终端已打开 · 当前程序：{}\n目录：{}\n/attach 可在电脑接管。",
            status.agent.display_name(),
            status.runner.workspace.display()
        ),
        None => "当前没有打开的终端。".into(),
    };
    let _ = bot.send_message(chat_id, message).await;
}
async fn send_screen(bot: &Bot, chat_id: ChatId, state: &AppState) {
    let Some(status) = state.terminals.active(chat_id.0).await else {
        let _ = bot.send_message(chat_id, "当前没有打开的终端。").await;
        return;
    };
    match status.runner.capture_pane_public().await {
        Ok(screen) => {
            let _ = bot
                .send_message(
                    chat_id,
                    format!(
                        "<pre>{}</pre>",
                        presenter::html_escape(&presenter::tail_chars(
                            &presenter::strip_ansi(&screen),
                            3500
                        ))
                    ),
                )
                .parse_mode(ParseMode::Html)
                .await;
        }
        Err(error) => {
            let _ = bot
                .send_message(chat_id, format!("读取终端失败：{error}"))
                .await;
        }
    }
}

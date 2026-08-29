use std::{collections::HashMap, sync::Arc, time::Instant};

use teloxide::{
    prelude::*,
    types::{ChatAction, KeyboardButton, KeyboardMarkup, MessageId, ParseMode, ReplyMarkup},
};
use tokio::{sync::Mutex, task::JoinHandle};
use tracing::{info, warn};

use crate::{agent::AgentKind, cli::CliRunner, config::Config};

const CODEX: &str = "使用 Codex";
const CURSOR: &str = "使用 Cursor";
const CLAUDE: &str = "使用 Claude";
const CANCEL: &str = "仅聊天/取消";

struct TerminalSession {
    runner: Arc<CliRunner>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    watcher: JoinHandle<()>,
}

struct AppState {
    workspace: String,
    terminals: Mutex<HashMap<i64, TerminalSession>>,
    pending_prompts: Mutex<HashMap<i64, String>>,
}

pub async fn start_bot(config: Arc<Config>) {
    let state = Arc::new(AppState {
        workspace: config.workspace.path.clone(),
        terminals: Mutex::new(HashMap::new()),
        pending_prompts: Mutex::new(HashMap::new()),
    });
    let allowed_ids = config.telegram.allowed_user_ids.clone();
    let bot = Bot::new(&config.telegram.token);
    info!(workspace = %state.workspace, "ChatCLI interactive bot starting");
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

async fn handle_message(bot: Bot, msg: Message, state: Arc<AppState>, allowed_ids: Vec<i64>) {
    let user_id = msg
        .from
        .as_ref()
        .map(|user| user.id.0 as i64)
        .unwrap_or_default();
    let chat_id = msg.chat.id;
    if !allowed_ids.contains(&user_id) {
        warn!(user_id, "Rejected unauthorized Telegram message");
        return;
    }
    let Some(text) = msg.text().map(str::trim).filter(|text| !text.is_empty()) else {
        return;
    };

    if matches!(text, "/start" | "/help") {
        let _ = bot
            .send_message(chat_id, help_text())
            .reply_markup(main_keyboard())
            .await;
        return;
    }
    if matches!(text, "/status" | "状态") {
        send_status(&bot, chat_id, &state).await;
        return;
    }
    if matches!(text, "/close" | "/exit" | "关闭终端" | "退出终端") {
        close_terminal(&bot, chat_id, &state).await;
        return;
    }
    if matches!(text, "/attach" | "打开电脑终端") {
        attach_local(&bot, chat_id, &state).await;
        return;
    }
    if matches!(text, "/stop" | "/c" | "中断" | "Ctrl+C" | "ctrl+c") {
        interrupt(&bot, chat_id, &state).await;
        return;
    }
    if text == "/log" {
        send_log(&bot, chat_id, &state).await;
        return;
    }
    if let Some((kind, prompt)) = parse_cli_command(text) {
        open_terminal(&bot, chat_id, Arc::clone(&state), kind, prompt).await;
        return;
    }
    if matches!(
        text,
        "/open" | "/terminal" | "打开" | "打开终端" | "启动终端"
    ) {
        show_chooser(&bot, chat_id, &state, None).await;
        return;
    }
    if let Some(kind) = chosen_kind(text) {
        let prompt = state.pending_prompts.lock().await.remove(&chat_id.0);
        open_terminal(&bot, chat_id, Arc::clone(&state), kind, prompt).await;
        return;
    }
    if text == CANCEL {
        state.pending_prompts.lock().await.remove(&chat_id.0);
        let _ = bot
            .send_message(
                chat_id,
                "已取消。ChatCLI 只控制第三方 CLI；请用 /cli codex 或 /cli cursor 开始。",
            )
            .await;
        return;
    }
    if let Some(runner) = active_runner(&state, chat_id.0).await {
        info!(chat_id = chat_id.0, agent = %runner.kind, "Forwarding Telegram input to terminal");
        if let Err(error) = runner.send_line(text).await {
            let _ = bot
                .send_message(chat_id, format!("发送失败：{error}"))
                .await;
        }
    } else {
        show_chooser(&bot, chat_id, &state, Some(text.to_owned())).await;
    }
}

fn parse_cli_command(text: &str) -> Option<(AgentKind, Option<String>)> {
    let rest = text.strip_prefix("/cli ")?.trim();
    let mut words = rest.splitn(2, char::is_whitespace);
    let kind = words.next()?.parse().ok()?;
    let prompt = words
        .next()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned);
    Some((kind, prompt))
}

fn chosen_kind(text: &str) -> Option<AgentKind> {
    match text {
        CODEX => Some(AgentKind::Codex),
        CURSOR => Some(AgentKind::Cursor),
        CLAUDE => Some(AgentKind::Claude),
        _ => None,
    }
}

async fn show_chooser(bot: &Bot, chat_id: ChatId, state: &AppState, prompt: Option<String>) {
    if let Some(prompt) = prompt {
        state.pending_prompts.lock().await.insert(chat_id.0, prompt);
    }
    let _ = bot
        .send_message(chat_id, "当前没有 chatcli 终端。请选择要打开的交互式 CLI：")
        .reply_markup(chooser_keyboard())
        .await;
}

async fn open_terminal(
    bot: &Bot,
    chat_id: ChatId,
    state: Arc<AppState>,
    kind: AgentKind,
    prompt: Option<String>,
) {
    close_silently(chat_id, &state).await;
    let name = format!("chatcli-tg-{}-{}", chat_id.0.unsigned_abs(), kind.as_str());
    let runner = Arc::new(CliRunner::new(kind, state.workspace.clone(), name));
    let _ = bot.send_chat_action(chat_id, ChatAction::Typing).await;
    if let Err(error) = runner.start_agent().await {
        let _ = bot
            .send_message(
                chat_id,
                format!("启动 {} 失败：{error}", kind.display_name()),
            )
            .await;
        return;
    }
    start_watcher(bot.clone(), chat_id, Arc::clone(&runner), &state).await;
    let _ = bot
        .send_message(
            chat_id,
            format!(
                "已打开 {} 交互终端。后续消息会直接发给它；/attach 可在 Mac 上接入同一会话。",
                kind.display_name()
            ),
        )
        .await;
    if let Some(prompt) = prompt {
        tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
        if let Err(error) = runner.send_line(&prompt).await {
            let _ = bot
                .send_message(chat_id, format!("发送初始任务失败：{error}"))
                .await;
        }
    }
}

async fn active_runner(state: &AppState, chat_id: i64) -> Option<Arc<CliRunner>> {
    state
        .terminals
        .lock()
        .await
        .get(&chat_id)
        .map(|session| Arc::clone(&session.runner))
}

async fn interrupt(bot: &Bot, chat_id: ChatId, state: &AppState) {
    match active_runner(state, chat_id.0).await {
        Some(runner) => {
            let _ = runner.send_raw_keys("C-c").await;
            let _ = bot.send_message(chat_id, "已发送 Ctrl+C。").await;
        }
        None => {
            let _ = bot.send_message(chat_id, "当前没有打开的终端。").await;
        }
    }
}

async fn close_terminal(bot: &Bot, chat_id: ChatId, state: &AppState) {
    let text = if close_silently(chat_id, state).await {
        "已关闭当前 chatcli 终端。"
    } else {
        "当前没有打开的终端。"
    };
    let _ = bot.send_message(chat_id, text).await;
}

async fn close_silently(chat_id: ChatId, state: &AppState) -> bool {
    let session = state.terminals.lock().await.remove(&chat_id.0);
    let Some(mut session) = session else {
        return false;
    };
    if let Some(stop) = session.stop.take() {
        let _ = stop.send(());
    }
    session.watcher.abort();
    let _ = session.runner.kill().await;
    true
}

async fn attach_local(bot: &Bot, chat_id: ChatId, state: &AppState) {
    let Some(runner) = active_runner(state, chat_id.0).await else {
        let _ = bot.send_message(chat_id, "当前没有打开的终端。").await;
        return;
    };
    let text = match runner.open_local_terminal().await {
        Ok(()) => "已在 Mac Terminal 中 attach 到同一 tmux 会话。".to_string(),
        Err(error) => format!("打开本机终端失败：{error}"),
    };
    let _ = bot.send_message(chat_id, text).await;
}

async fn send_status(bot: &Bot, chat_id: ChatId, state: &AppState) {
    let text = match active_runner(state, chat_id.0).await {
        Some(runner) => format!(
            "当前终端：{}\nCLI：{}\n项目：{}\n本机接入：/attach",
            runner.tmux_session,
            runner.kind.display_name(),
            runner.workspace.display()
        ),
        None => "当前没有打开的 chatcli 终端。".to_string(),
    };
    let _ = bot.send_message(chat_id, text).await;
}

async fn send_log(bot: &Bot, chat_id: ChatId, state: &AppState) {
    let Some(runner) = active_runner(state, chat_id.0).await else {
        let _ = bot.send_message(chat_id, "当前没有打开的终端。").await;
        return;
    };
    match runner.capture_pane_public().await {
        Ok(text) => {
            let _ =
                publish_terminal_view(bot, chat_id, None, &last_n_lines(&strip_ansi(&text), 80))
                    .await;
        }
        Err(error) => {
            let _ = bot
                .send_message(chat_id, format!("读取日志失败：{error}"))
                .await;
        }
    }
}

async fn start_watcher(bot: Bot, chat_id: ChatId, runner: Arc<CliRunner>, state: &AppState) {
    let (stop, mut stop_rx) = tokio::sync::oneshot::channel();
    let runner_for_watcher = Arc::clone(&runner);
    let watcher = tokio::spawn(async move {
        watch_pane(bot, chat_id, runner_for_watcher, &mut stop_rx).await;
    });
    state.terminals.lock().await.insert(
        chat_id.0,
        TerminalSession {
            runner,
            stop: Some(stop),
            watcher,
        },
    );
}

async fn watch_pane(
    bot: Bot,
    chat_id: ChatId,
    runner: Arc<CliRunner>,
    stop: &mut tokio::sync::oneshot::Receiver<()>,
) {
    let mut last_view = String::new();
    let mut live_message = None;
    let mut last_edit = Instant::now() - tokio::time::Duration::from_secs(2);
    loop {
        tokio::select! { _ = &mut *stop => break, _ = tokio::time::sleep(tokio::time::Duration::from_millis(600)) => {} }
        let Ok(pane) = runner.capture_pane_public().await else {
            continue;
        };
        let view = last_n_lines(&strip_ansi(&pane), 40);
        if view.is_empty()
            || view == last_view
            || last_edit.elapsed() < tokio::time::Duration::from_millis(1100)
        {
            continue;
        }
        last_view = view.clone();
        last_edit = Instant::now();
        live_message = publish_terminal_view(&bot, chat_id, live_message, &view).await;
    }
}

async fn publish_terminal_view(
    bot: &Bot,
    chat_id: ChatId,
    message_id: Option<MessageId>,
    text: &str,
) -> Option<MessageId> {
    let body = format!("<b>[终端]</b>\n<pre>{}</pre>", html_escape(text));
    if let Some(message_id) = message_id {
        if bot
            .edit_message_text(chat_id, message_id, &body)
            .parse_mode(ParseMode::Html)
            .await
            .is_ok()
        {
            return Some(message_id);
        }
    }
    bot.send_message(chat_id, body)
        .parse_mode(ParseMode::Html)
        .await
        .ok()
        .map(|message| message.id)
}

fn chooser_keyboard() -> ReplyMarkup {
    ReplyMarkup::Keyboard(
        KeyboardMarkup::new([
            [KeyboardButton::new(CODEX), KeyboardButton::new(CURSOR)],
            [KeyboardButton::new(CLAUDE), KeyboardButton::new(CANCEL)],
        ])
        .resize_keyboard()
        .persistent(),
    )
}
fn main_keyboard() -> ReplyMarkup {
    ReplyMarkup::Keyboard(
        KeyboardMarkup::new([
            [
                KeyboardButton::new("/cli codex"),
                KeyboardButton::new("/cli cursor"),
            ],
            [
                KeyboardButton::new("/status"),
                KeyboardButton::new("/attach"),
            ],
            [KeyboardButton::new("/close"), KeyboardButton::new("/log")],
        ])
        .resize_keyboard()
        .persistent(),
    )
}
fn help_text() -> &'static str {
    "ChatCLI — Telegram 远程交互式 CLI\n\n/cli codex [任务]\n/cli cursor [任务]\n/cli claude [任务]\n/open：选择并打开 CLI\n/status：查看当前终端\n/attach：在 Mac Terminal 中接入\n/log：查看日志\n/stop：发送 Ctrl+C\n/close：关闭当前终端\n\n没有终端时直接发任务，会先询问用哪个 CLI 打开。"
}
fn last_n_lines(text: &str, n: usize) -> String {
    let lines: Vec<_> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..]
        .join("\n")
        .trim_end()
        .to_string()
}
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        if ch != '\r' {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_cli_command() {
        let (kind, prompt) = parse_cli_command("/cli codex 修复测试").unwrap();
        assert_eq!(kind, AgentKind::Codex);
        assert_eq!(prompt.as_deref(), Some("修复测试"));
    }
}

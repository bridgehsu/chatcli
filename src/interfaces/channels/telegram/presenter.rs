use crate::{
    agent::AgentKind,
    infrastructure::{SessionRecord, SessionState},
};
use teloxide::types::{KeyboardRemove, ReplyMarkup};

/// ChatCLI 所有操作通过聊天消息完成，不展示 Telegram 底部键盘。
fn remove_keyboard() -> ReplyMarkup {
    ReplyMarkup::KeyboardRemove(KeyboardRemove::new())
}
pub fn initial_keyboard() -> ReplyMarkup {
    remove_keyboard()
}
pub fn cli_home_keyboard(kind: AgentKind) -> ReplyMarkup {
    let _ = kind;
    remove_keyboard()
}
pub fn create_keyboard() -> ReplyMarkup {
    remove_keyboard()
}
pub fn active_keyboard(kind: AgentKind) -> ReplyMarkup {
    let _ = kind;
    remove_keyboard()
}
pub fn list_keyboard(records: &[SessionRecord]) -> ReplyMarkup {
    let _ = records;
    remove_keyboard()
}
pub fn help_text() -> &'static str {
    "ChatCLI\n\n选择 CodeX 或 Cursor 后可新建并管理多个会话。\nShell 中的自然语言会先生成命令，确认后才执行；以 $ 或 ! 开头可直接发送 Shell 命令。\n/reset：清空当前聊天的 Agent 会话、上下文和受管理终端。"
}
pub fn session_list_text(records: &[SessionRecord], current: Option<&str>) -> String {
    if records.is_empty() {
        return "当前 Agent 会话还没有终端。".to_owned();
    }
    let entries = records
        .iter()
        .map(|r| {
            let marker = if current == Some(r.id.as_str()) {
                "● 当前"
            } else {
                "○"
            };
            let status = match r.status {
                SessionState::Running => "运行中",
                SessionState::Stopped => "已结束",
                SessionState::Missing => "已失效",
            };
            format!(
                "{} · {}\n{}\n{} · {}\nID: {}",
                marker,
                r.kind.display_name(),
                r.workspace
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("工作目录"),
                r.workspace.display(),
                status,
                r.id
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "当前 Agent Session 的终端列表\n\n{}\n\n发送 /terminal 终端ID 可切换运行中的终端。",
        entries
    )
}
pub fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
pub fn strip_ansi(text: &str) -> String {
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
pub fn tail_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.into()
    } else {
        text.chars()
            .rev()
            .take(max)
            .collect::<String>()
            .chars()
            .rev()
            .collect()
    }
}

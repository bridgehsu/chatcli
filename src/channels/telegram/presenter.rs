use crate::agent::intent::commands::*;
use crate::{
    agent::AgentKind,
    session::{SessionRecord, SessionState},
};
use teloxide::types::{KeyboardButton, KeyboardMarkup, ReplyMarkup};

fn keyboard(rows: Vec<Vec<&str>>) -> ReplyMarkup {
    ReplyMarkup::Keyboard(
        KeyboardMarkup::new(
            rows.into_iter()
                .map(|row| row.into_iter().map(KeyboardButton::new).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
        )
        .resize_keyboard()
        .persistent(),
    )
}
pub fn initial_keyboard() -> ReplyMarkup {
    keyboard(vec![vec![CODEX, CURSOR]])
}
pub fn cli_home_keyboard(kind: AgentKind) -> ReplyMarkup {
    match kind {
        AgentKind::Codex => keyboard(vec![vec![NEW_CODEX, LIST_CODEX], vec![RESET]]),
        AgentKind::Cursor => keyboard(vec![vec![NEW_CURSOR, LIST_CURSOR], vec![RESET]]),
    }
}
pub fn create_keyboard() -> ReplyMarkup {
    keyboard(vec![vec![CANCEL], vec![RESET]])
}
pub fn active_keyboard(kind: AgentKind) -> ReplyMarkup {
    match kind {
        AgentKind::Codex => keyboard(vec![
            vec![LIST_CODEX, NEW_CODEX],
            vec![SCREEN, ATTACH],
            vec![STOP, CLOSE],
            vec![RESET],
        ]),
        AgentKind::Cursor => keyboard(vec![
            vec![LIST_CURSOR, NEW_CURSOR],
            vec![SCREEN, ATTACH],
            vec![STOP, CLOSE],
            vec![RESET],
        ]),
    }
}
pub fn list_keyboard(records: &[SessionRecord]) -> ReplyMarkup {
    let mut rows = records
        .iter()
        .filter(|r| r.status == SessionState::Running)
        .take(12)
        .map(|r| vec![format!("/session {}", r.id)])
        .collect::<Vec<_>>();
    rows.push(vec![BACK.to_owned()]);
    rows.push(vec![RESET.to_owned()]);
    ReplyMarkup::Keyboard(
        KeyboardMarkup::new(
            rows.into_iter()
                .map(|row| row.into_iter().map(KeyboardButton::new).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
        )
        .resize_keyboard()
        .persistent(),
    )
}
pub fn help_text() -> &'static str {
    "ChatCLI\n\n选择 Codex 或 Cursor 后可新建并管理多个会话。\n当前会话中的普通消息会直接输入 CLI。\n/debug_reset：仅重置机器人 UI，不关闭后台会话。"
}
pub fn session_list_text(
    records: &[SessionRecord],
    current: Option<&str>,
    kind: AgentKind,
) -> String {
    if records.is_empty() {
        return format!("{} 暂无会话。", kind.display_name());
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
                "{} · {}\n{} · {}\nID: {}",
                marker,
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
        "{} 所有会话\n\n{}\n\n点击下方 /session 编号切换运行中的会话。",
        kind.display_name(),
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

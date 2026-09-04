use crate::{
    agent::AgentKind,
    manager::{Session, SessionRecord, SessionState},
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
/// Agent Session 切换键盘；使用独立前缀，避免与 tmux CLI Session 混淆。
pub fn agent_session_keyboard(sessions: &[Session]) -> ReplyMarkup {
    let _ = sessions;
    remove_keyboard()
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
        "{} 所有会话\n\n{}\n\n点击下方 /manager 编号切换运行中的会话。",
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

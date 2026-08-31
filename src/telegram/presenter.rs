use teloxide::types::{KeyboardButton, KeyboardMarkup, ReplyMarkup};

use super::commands::{CANCEL, OPEN_CODEX, OPEN_CURSOR, USE_HOME};

pub fn open_keyboard() -> ReplyMarkup {
    ReplyMarkup::Keyboard(
        KeyboardMarkup::new([
            [
                KeyboardButton::new(OPEN_CODEX),
                KeyboardButton::new(OPEN_CURSOR),
            ],
            [KeyboardButton::new(CANCEL), KeyboardButton::new("/help")],
        ])
        .resize_keyboard()
        .persistent(),
    )
}
pub fn main_keyboard() -> ReplyMarkup {
    ReplyMarkup::Keyboard(
        KeyboardMarkup::new([
            [KeyboardButton::new("/open"), KeyboardButton::new("/status")],
            [
                KeyboardButton::new("/screen"),
                KeyboardButton::new("/attach"),
            ],
            [KeyboardButton::new("/stop"), KeyboardButton::new("/close")],
        ])
        .resize_keyboard()
        .persistent(),
    )
}

pub fn workspace_keyboard() -> ReplyMarkup {
    ReplyMarkup::Keyboard(
        KeyboardMarkup::new(vec![
            vec![KeyboardButton::new(USE_HOME)],
            vec![KeyboardButton::new(CANCEL), KeyboardButton::new("/help")],
        ])
        .resize_keyboard()
        .persistent(),
    )
}

pub fn help_text() -> &'static str {
    "ChatCLI — 聊天终端展示与控制工具\n\n打开 CLI：/open\n直接启动：/cli codex [任务]、/cli cursor [任务]\n选择 CLI 后发送目录路径，或选择 Home (~)。\n查看屏幕：/screen\n电脑接管：/attach\n中断：/stop\n关闭：/close"
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

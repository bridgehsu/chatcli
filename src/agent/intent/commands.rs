use crate::agent::AgentKind;

pub const CODEX: &str = "Codex";
pub const CURSOR: &str = "Cursor";
pub const NEW_CODEX: &str = "新建 Codex 会话";
pub const NEW_CURSOR: &str = "新建 Cursor 会话";
pub const LIST_CODEX: &str = "Codex 所有会话";
pub const LIST_CURSOR: &str = "Cursor 所有会话";
pub const SCREEN: &str = "查看屏幕";
pub const ATTACH: &str = "切换到终端窗口";
pub const STOP: &str = "中断当前任务";
pub const CLOSE: &str = "结束当前会话";
pub const CANCEL: &str = "取消创建";
pub const RESET: &str = "🛠 重置调试状态";
pub const BACK: &str = "返回";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Status,
    Stop,
    Close,
    Attach,
    Screen,
    CliHome(AgentKind),
    NewSession(AgentKind),
    List(AgentKind),
    SelectSession(String),
    Cancel,
    Reset,
    Back,
    Text(String),
}

pub fn parse(text: &str) -> Command {
    match text {
        "/start" | "/help" => Command::Help,
        "/status" => Command::Status,
        "/stop" | "/c" | STOP => Command::Stop,
        "/close" | "/exit" | CLOSE => Command::Close,
        "/attach" | ATTACH => Command::Attach,
        "/screen" | "/log" | SCREEN => Command::Screen,
        "/debug_reset" | RESET => Command::Reset,
        CANCEL => Command::Cancel,
        BACK => Command::Back,
        CODEX => Command::CliHome(AgentKind::Codex),
        CURSOR => Command::CliHome(AgentKind::Cursor),
        NEW_CODEX => Command::NewSession(AgentKind::Codex),
        NEW_CURSOR => Command::NewSession(AgentKind::Cursor),
        LIST_CODEX => Command::List(AgentKind::Codex),
        LIST_CURSOR => Command::List(AgentKind::Cursor),
        _ => text
            .strip_prefix("/session ")
            .map(|id| Command::SelectSession(id.trim().to_owned()))
            .unwrap_or_else(|| Command::Text(text.to_owned())),
    }
}

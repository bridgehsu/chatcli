use crate::agent::AgentKind;

pub const OPEN_CODEX: &str = "启动 Codex";
pub const OPEN_CURSOR: &str = "启动 Cursor";
pub const USE_HOME: &str = "使用 Home 目录 (~)";
pub const CANCEL: &str = "取消";

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Status,
    Stop,
    Close,
    Attach,
    Screen,
    OpenMenu,
    OpenAgent(AgentKind, Option<String>),
    ChooseAgent(AgentKind),
    UseHome,
    Cancel,
    Text(String),
}

pub fn parse(text: &str) -> Command {
    match text {
        "/start" | "/help" => return Command::Help,
        "/status" | "状态" => return Command::Status,
        "/stop" | "/c" | "中断" | "Ctrl+C" => return Command::Stop,
        "/close" | "/exit" | "关闭终端" => return Command::Close,
        "/attach" | "在电脑打开" => return Command::Attach,
        "/screen" | "/log" => return Command::Screen,
        "/open" | "打开" => return Command::OpenMenu,
        CANCEL => return Command::Cancel,
        OPEN_CODEX => return Command::ChooseAgent(AgentKind::Codex),
        OPEN_CURSOR => return Command::ChooseAgent(AgentKind::Cursor),
        USE_HOME => return Command::UseHome,
        _ => {}
    }
    if let Some((kind, prompt)) = parse_cli(text) {
        Command::OpenAgent(kind, prompt)
    } else {
        Command::Text(text.to_owned())
    }
}

fn parse_cli(text: &str) -> Option<(AgentKind, Option<String>)> {
    let rest = text.strip_prefix("/cli ")?.trim();
    let mut parts = rest.splitn(2, char::is_whitespace);
    let kind = parts.next()?.parse().ok()?;
    Some((
        kind,
        parts
            .next()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_agent_prompt() {
        assert_eq!(
            parse("/cli codex fix"),
            Command::OpenAgent(AgentKind::Codex, Some("fix".into()))
        );
    }

    #[test]
    fn opens_cli_picker() {
        assert_eq!(parse("/open"), Command::OpenMenu);
    }
}

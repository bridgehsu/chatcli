//! 意图识别：将文本转换为配置中的意图名称及少量参数。

mod catalog;
pub mod commands;
mod model;
mod normalize;

use std::sync::Arc;

use crate::{agent::AgentKind, infrastructure::config::RouterConfig};
use normalize::prepare;

pub use catalog::IntentCatalog;

/// 已识别的意图。名称来自 `config/intents.yaml`，而非 Rust 枚举。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intent {
    pub name: String,
    pub cli: Option<AgentKind>,
    pub argument: Option<String>,
    pub route: IntentRoute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentRoute {
    Action,
    WorkspaceInput,
    CliInput,
}

/// Intent 层只需要的最小会话输入模式，避免反向依赖 Agent 的 CurrentSession。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Idle,
    WaitingWorkspace,
    ActiveCli,
}

impl Intent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cli: None,
            argument: None,
            route: IntentRoute::Action,
        }
    }

    pub fn choose_cli(kind: AgentKind) -> Self {
        Self {
            name: "choose_cli".to_owned(),
            cli: Some(kind),
            argument: None,
            route: IntentRoute::Action,
        }
    }

    pub fn with_cli(mut self, cli: Option<AgentKind>) -> Self {
        self.cli = cli;
        self
    }

    fn workspace_input(input: &str) -> Self {
        Self {
            name: "workspace_input".to_owned(),
            cli: None,
            argument: Some(input.to_owned()),
            route: IntentRoute::WorkspaceInput,
        }
    }

    fn cli_input(input: &str) -> Self {
        Self {
            name: "cli_input".to_owned(),
            cli: None,
            argument: Some(input.to_owned()),
            route: IntentRoute::CliInput,
        }
    }
}

/// 精确规则优先；未命中时由模型按 YAML 目录分类。
pub struct Recognizer {
    router: Arc<RouterConfig>,
    catalog: Arc<IntentCatalog>,
}

impl Recognizer {
    pub fn new(router: Arc<RouterConfig>, catalog: Arc<IntentCatalog>) -> Self {
        Self { router, catalog }
    }

    /// 确定性意图规则。命中后必须优先于会话状态处理，避免控制语句被转发给 CLI。
    fn rule(&self, input: &str) -> Option<Intent> {
        let input = prepare(input);
        if let Some(id) = input.text.strip_prefix("/agent_session ") {
            let id = id.trim();
            if !id.is_empty() {
                return Some(Intent {
                    name: "switch_agent_session".to_owned(),
                    cli: None,
                    argument: Some(id.to_owned()),
                    route: IntentRoute::Action,
                });
            }
        }
        match input.lower.as_str() {
            "打开终端" | "开启终端" => Some(Intent::new("open_terminal")),
            "关闭终端" | "结束终端" => Some(Intent::new("close_terminal")),
            "开发会话" | "新建会话" => Some(Intent::new("new_session")),
            "关闭会话" | "结束会话" => Some(Intent::new("close_session")),
            "会话列表" | "获取会话列表" => Some(Intent::new("list_session")),
            "切换会话" | "切换会话列表" => Some(Intent::new("switch_session")),
            "自我介绍" | "介绍一下你自己" | "你是谁" | "你能做什么" => {
                Some(Intent::new("self_introduction"))
            }
            "codex" => Some(Intent::choose_cli(AgentKind::Codex)),
            "cursor" => Some(Intent::choose_cli(AgentKind::Cursor)),
            "/reset" => Some(Intent::new("reset")),
            _ => None,
        }
    }

    /// 仅对没有命中确定性规则的普通文本调用模型兜底。
    async fn fallback(&self, input: &str) -> Intent {
        let input = prepare(input);
        model::recognize(&input, &self.router, &self.catalog)
            .await
            .unwrap_or_else(|| Intent::new("unknown"))
    }

    /// 完整识别入口，便于不需要会话状态编排的调用方直接使用。
    pub async fn run(&self, input: &str, mode: InputMode) -> Intent {
        // 如果当前没有开启会话

        // 如果当前开启会话，没有打开对应的cli

        // 如果当前开启了会话，打开对应的cli

        if let Some(intent) = self.rule(input) {
            return intent;
        }
        match mode {
            InputMode::WaitingWorkspace => Intent::workspace_input(input),
            InputMode::ActiveCli => Intent::cli_input(input),
            InputMode::Idle => self.fallback(input).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn recognizes_open_terminal() {
        let intent = Recognizer::new(
            Arc::new(RouterConfig::default()),
            Arc::new(IntentCatalog::default()),
        )
        .run("打开终端", InputMode::Idle)
        .await;
        assert_eq!(intent.name, "open_terminal");
    }
}

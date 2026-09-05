//! 意图识别：将文本转换为配置中的意图名称及少量参数。

mod catalog;
mod model;
mod normalize;
mod profile;

use std::sync::Arc;

use crate::{
    agent::AgentKind, app::agent_runtime::CurrentSession, infrastructure::config::RouterConfig,
};
use normalize::prepare;

pub use catalog::IntentCatalog;
pub use profile::{FallbackPolicy, RecognitionProfile};

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
    /// Shell 中的自然语言请求，需先生成候选命令并等待用户确认。
    ShellRequest,
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

    fn shell_request(input: &str) -> Self {
        Self {
            name: "shell_request".to_owned(),
            cli: None,
            argument: Some(input.to_owned()),
            route: IntentRoute::ShellRequest,
        }
    }

    fn shell_edit(command: &str) -> Self {
        Self {
            name: "shell_edit".to_owned(),
            cli: None,
            argument: Some(command.to_owned()),
            route: IntentRoute::Action,
        }
    }

    fn agent_chat(input: &str) -> Self {
        Self {
            name: "agent_chat".to_owned(),
            cli: None,
            argument: Some(input.to_owned()),
            route: IntentRoute::Action,
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
        if let Some(id) = input.text.strip_prefix("/terminal ") {
            let id = id.trim();
            if !id.is_empty() {
                return Some(Intent {
                    name: "switch_terminal_id".to_owned(),
                    cli: None,
                    argument: Some(id.to_owned()),
                    route: IntentRoute::Action,
                });
            }
        }
        match input.lower.as_str() {
            "打开终端" | "开启终端" => Some(Intent::new("open_terminal")),
            "关闭终端" | "结束终端" => Some(Intent::new("close_terminal")),
            "终端列表" | "查看终端" | "获取终端列表" => {
                Some(Intent::new("list_terminal"))
            }
            "切换终端" | "切换终端列表" => Some(Intent::new("switch_terminal")),
            "开发会话" | "新建会话" => Some(Intent::new("new_session")),
            "关闭会话" | "结束会话" => Some(Intent::new("close_session")),
            "会话列表" | "获取会话列表" => Some(Intent::new("list_session")),
            "切换会话" | "切换会话列表" => Some(Intent::new("switch_session")),
            "自我介绍" | "介绍一下你自己" | "你是谁" | "你能做什么" => {
                Some(Intent::new("self_introduction"))
            }
            "取消" | "不选了" => Some(Intent::new("cancel")),
            "帮助" | "怎么用" | "/help" => Some(Intent::new("help")),
            "codex" => Some(Intent::choose_cli(AgentKind::Codex)),
            "cursor" => Some(Intent::choose_cli(AgentKind::Cursor)),
            "/reset" => Some(Intent::new("reset")),
            _ => None,
        }
    }

    /// 仅对没有命中确定性规则的普通文本调用模型兜底。
    async fn fallback(&self, input: &str, profile: &RecognitionProfile) -> Intent {
        let input = prepare(input);
        // 等待目录是强制流程：普通文本只作为目录输入，不调用模型聊天或控制终端。
        if profile.fallback == FallbackPolicy::WorkspaceInput {
            return Intent::workspace_input(&input.text);
        }
        model::recognize(&input, &self.router, &self.catalog, profile.allowed)
            .await
            .unwrap_or_else(|| match profile.fallback {
                FallbackPolicy::AgentChat => Intent::agent_chat(&input.text),
                FallbackPolicy::WorkspaceInput => Intent::workspace_input(&input.text),
                FallbackPolicy::TerminalInput => Intent::cli_input(&input.text),
                FallbackPolicy::ShellRequest => Intent::shell_request(&input.text),
                FallbackPolicy::WaitingShellConfirmation => {
                    Intent::new("shell_waiting_confirmation")
                }
            })
    }

    /// 完整识别入口：先处理确定性规则，再依据当前会话决定普通文本的去向。
    pub async fn run(&self, input: &str, current: &CurrentSession) -> Intent {
        let profile = RecognitionProfile::from_current(current);
        let normalized = prepare(input);
        if profile.kind == profile::ProfileKind::WaitingShellConfirmation {
            if let Some(command) = normalized
                .text
                .strip_prefix("修改为：")
                .or_else(|| normalized.text.strip_prefix("修改为:"))
                .or_else(|| normalized.text.strip_prefix("修改："))
                .or_else(|| normalized.text.strip_prefix("修改:"))
                .map(str::trim)
                .filter(|command| !command.is_empty())
            {
                return Intent::shell_edit(command);
            }
            return match normalized.lower.as_str() {
                "确认" | "执行" | "确认执行" => Intent::new("shell_confirm"),
                "取消" | "不执行" => Intent::new("shell_cancel"),
                _ => {
                    if let Some(intent) = self.rule(input) {
                        if profile.allows(&intent.name) {
                            intent
                        } else {
                            Intent::new("shell_waiting_confirmation")
                        }
                    } else {
                        Intent::new("shell_waiting_confirmation")
                    }
                }
            };
        }
        // `$` 或 `!` 表示用户明确要手写 Shell，跳过模型建议与确认流程。
        if profile.kind == profile::ProfileKind::Shell {
            if let Some(command) = normalized
                .text
                .strip_prefix('$')
                .or_else(|| normalized.text.strip_prefix('!'))
                .map(str::trim)
                .filter(|command| !command.is_empty())
            {
                return Intent::cli_input(command);
            }
        }
        if let Some(intent) = self.rule(input) {
            if profile.allows(&intent.name) {
                return intent;
            }
            return match profile.fallback {
                FallbackPolicy::WorkspaceInput => Intent::workspace_input(input),
                FallbackPolicy::TerminalInput => Intent::cli_input(input),
                FallbackPolicy::ShellRequest => Intent::shell_request(input),
                FallbackPolicy::WaitingShellConfirmation => {
                    Intent::new("shell_waiting_confirmation")
                }
                FallbackPolicy::AgentChat => Intent::agent_chat(input),
            };
        }
        self.fallback(input, &profile).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::agent_runtime::TerminalState, domain::AgentSessionState};
    #[tokio::test]
    async fn recognizes_open_terminal() {
        let intent = Recognizer::new(
            Arc::new(RouterConfig::default()),
            Arc::new(IntentCatalog::default()),
        )
        .run(
            "打开终端",
            &CurrentSession {
                id: None,
                state: None,
                terminal: TerminalState::None,
            },
        )
        .await;
        assert_eq!(intent.name, "open_terminal");
    }

    #[tokio::test]
    async fn shell_natural_language_requires_a_command_proposal() {
        let intent = Recognizer::new(
            Arc::new(RouterConfig::default()),
            Arc::new(IntentCatalog::default()),
        )
        .run(
            "查看当前目录下有哪些文件",
            &CurrentSession {
                id: Some("agent-1".to_owned()),
                state: None,
                terminal: TerminalState::Active {
                    kind: crate::agent::TerminalKind::Shell,
                    terminal_id: "shell-1".to_owned(),
                },
            },
        )
        .await;
        assert_eq!(intent.route, IntentRoute::ShellRequest);
    }

    #[tokio::test]
    async fn pending_shell_command_accepts_edit_and_confirmation_only() {
        let recognizer = Recognizer::new(
            Arc::new(RouterConfig::default()),
            Arc::new(IntentCatalog::default()),
        );
        let current = CurrentSession {
            id: Some("agent-1".to_owned()),
            state: Some(AgentSessionState::WaitingShellCommand {
                terminal_session_id: "shell-1".to_owned(),
                command: "ls".to_owned(),
                description: "列目录".to_owned(),
            }),
            terminal: TerminalState::Active {
                kind: crate::agent::TerminalKind::Shell,
                terminal_id: "shell-1".to_owned(),
            },
        };
        let edit = recognizer.run("修改为：ls -lah", &current).await;
        assert_eq!(edit.name, "shell_edit");
        assert_eq!(edit.argument.as_deref(), Some("ls -lah"));
        assert_eq!(recognizer.run("确认", &current).await.name, "shell_confirm");
        assert_eq!(
            recognizer.run("查看目录", &current).await.name,
            "shell_waiting_confirmation"
        );
    }
}

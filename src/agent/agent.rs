use crate::{
    agent::{
        actions,
        intent::{self, InputMode, IntentRoute},
        AgentKind, Resolver, TerminalTool, WorkspaceSearchTool,
    },
    infrastructure::config::RouterConfig,
    manager::{AgentSessionState as SessionState, Session, SessionManager, TerminalManager},
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use teloxide::{prelude::*, types::ChatId};
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Telegram CLI 会话的业务编排者：识别输入、读取状态并分发动作。
pub struct Agent {
    pub(super) terminals: TerminalManager,
    pub(super) pending: Mutex<HashMap<i64, PendingLaunch>>,
    pub(super) selected: Mutex<HashMap<i64, AgentKind>>,
    pub(super) workspace: WorkspaceSearchTool,
    pub(super) terminal: TerminalTool,
    sessions: SessionManager,
    resolver: Resolver,
    intent: intent::Recognizer,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct PendingLaunch {
    pub(crate) kind: Option<AgentKind>,
    pub(crate) candidates: Vec<PathBuf>,
    #[serde(default)]
    pub(crate) workspace: Option<PathBuf>,
}

/// 当前 chat 正在使用的 Agent Session 运行时视图。
///
/// 它把持久化的 SessionState 与实际可用的 tmux 终端合并，供一次消息处理复用。
#[derive(Debug, Clone)]
pub(crate) struct CurrentSession {
    pub(crate) id: Option<String>,
    pub(crate) state: Option<SessionState>,
    pub(crate) cli: CliState,
}

/// 当前 Session 与 CLI 的关系：未选择、已选择但未聊天、或正在聊天。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliState {
    None,
    Selected(AgentKind),
    Active {
        kind: AgentKind,
        terminal_id: String,
    },
}

impl Agent {
    pub async fn new(
        home: PathBuf,
        router: Arc<RouterConfig>,
        catalog: Arc<intent::IntentCatalog>,
    ) -> anyhow::Result<Self> {
        let terminals = TerminalManager::new(home.clone());
        terminals.restore().await;
        let sessions = SessionManager::new(&home);
        let restored = sessions.restore().await;
        let mut selected = HashMap::new();
        let mut pending = HashMap::new();
        for (chat, session) in restored {
            match session.state {
                SessionState::Initial => {}
                SessionState::CliSelected { kind } => {
                    selected.insert(chat, kind);
                }
                SessionState::WaitingWorkspace {
                    kind,
                    candidates,
                    workspace,
                } => {
                    if let Some(kind) = kind {
                        selected.insert(chat, kind);
                    }
                    pending.insert(
                        chat,
                        PendingLaunch {
                            kind,
                            candidates,
                            workspace,
                        },
                    );
                }
                SessionState::ActiveTerminal { kind, .. } => {
                    selected.insert(chat, kind);
                }
            }
        }
        let resolver = Resolver::new(&router, Arc::clone(&catalog))?;
        Ok(Self {
            terminals,
            pending: Mutex::new(pending),
            selected: Mutex::new(selected),
            workspace: WorkspaceSearchTool::new(home),
            terminal: TerminalTool,
            sessions,
            resolver,
            intent: intent::Recognizer::new(router, catalog),
        })
    }

    pub(super) async fn persist_agent_session(&self, chat: ChatId) {
        let selected = self.selected.lock().await.get(&chat.0).copied();
        let pending = self.pending.lock().await.get(&chat.0).cloned();
        let terminal = self
            .terminals
            .active(chat.0)
            .await
            .map(|terminal| (terminal.agent, terminal.id));
        self.sessions
            .save_state(chat.0, selected, pending, terminal)
            .await;
    }

    pub(super) async fn new_session(&self, chat: ChatId) {
        self.sessions.new_session(chat.0).await;
    }

    pub(super) async fn record_context(
        &self,
        chat: ChatId,
        role: &str,
        content: impl Into<String>,
    ) {
        self.sessions.record(chat.0, role, content).await;
    }

    pub(super) async fn list_sessions(&self, chat: ChatId) -> (Vec<Session>, Option<String>) {
        self.sessions.list(chat.0).await
    }

    /// 一次性解析当前 Agent Session 与其关联 CLI 是否真的仍在运行。
    pub(crate) async fn get_current_session(&self, chat: ChatId) -> CurrentSession {
        let Some(session) = self.sessions.current(chat.0).await else {
            return CurrentSession {
                id: None,
                state: None,
                cli: CliState::None,
            };
        };
        let cli = match &session.state {
            SessionState::Initial => CliState::None,
            SessionState::CliSelected { kind } => CliState::Selected(*kind),
            SessionState::WaitingWorkspace { kind, .. } => {
                kind.map(CliState::Selected).unwrap_or(CliState::None)
            }
            SessionState::ActiveTerminal {
                kind,
                terminal_session_id,
            } => match self.terminals.active(chat.0).await {
                Some(active) if active.id == *terminal_session_id => CliState::Active {
                    kind: *kind,
                    terminal_id: terminal_session_id.clone(),
                },
                _ => CliState::Selected(*kind),
            },
        };
        CurrentSession {
            id: Some(session.id),
            state: Some(session.state),
            cli,
        }
    }

    /// 切换当前 Agent Session，并把持久化状态恢复到运行时内存。
    pub(super) async fn switch_session(&self, chat: ChatId, id: &str) -> Result<Session, String> {
        let session = self
            .sessions
            .select(chat.0, id)
            .await
            .ok_or_else(|| "未找到可切换的 Agent 会话。".to_owned())?;
        self.pending.lock().await.remove(&chat.0);
        self.selected.lock().await.remove(&chat.0);
        self.terminals.clear_current(chat.0).await;
        match &session.state {
            SessionState::Initial => {}
            SessionState::CliSelected { kind } => {
                self.selected.lock().await.insert(chat.0, *kind);
            }
            SessionState::WaitingWorkspace {
                kind,
                candidates,
                workspace,
            } => {
                if let Some(kind) = kind {
                    self.selected.lock().await.insert(chat.0, *kind);
                }
                self.pending.lock().await.insert(
                    chat.0,
                    PendingLaunch {
                        kind: *kind,
                        candidates: candidates.clone(),
                        workspace: workspace.clone(),
                    },
                );
            }
            SessionState::ActiveTerminal {
                kind,
                terminal_session_id,
            } => {
                if !self.terminals.select(chat.0, terminal_session_id).await {
                    return Err("该会话对应的 tmux 终端已结束或不可用。".to_owned());
                }
                self.selected.lock().await.insert(chat.0, *kind);
            }
        }
        self.record_context(
            chat,
            "system",
            format!("切换到 Agent Session：{}", session.id),
        )
        .await;
        Ok(session)
    }

    pub async fn run(self: Arc<Self>, bot: Bot, chat: ChatId, text: &str) {
        let current = self.get_current_session(chat).await;
        info!(
            chat_id = chat.0,
            session_id = ?current.id,
            cli = ?current.cli,
            "Resolved current Agent Session"
        );
        self.record_context(chat, "user", text).await;
        let intent = self.intent.run(text, current.input_mode()).await;
        self.dispatch_intent(&bot, chat, intent).await;
    }

    /// Intent 层已经决定输入去向；Agent 只执行相应的业务动作。
    async fn dispatch_intent(self: &Arc<Self>, bot: &Bot, chat: ChatId, intent: intent::Intent) {
        match intent.route {
            IntentRoute::Action => self.execute_intent(bot, chat, intent).await,
            IntentRoute::WorkspaceInput => {
                actions::workspace::launch(
                    bot,
                    chat,
                    Arc::clone(self),
                    intent.argument.as_deref().unwrap_or_default(),
                )
                .await;
            }
            IntentRoute::CliInput => {
                actions::session::forward(bot, chat, self, intent.argument.unwrap_or_default())
                    .await;
            }
        }
    }

    /// 记录识别结果并执行其配置动作；不负责判断输入来自规则还是模型。
    async fn execute_intent(self: &Arc<Self>, bot: &Bot, chat: ChatId, intent: intent::Intent) {
        self.record_context(chat, "intent", intent.name.clone())
            .await;
        info!(chat_id = chat.0, intent = %intent.name, cli = ?intent.cli, "Intent recognized");
        let decision = self.resolver.decide(intent);
        self.record_context(chat, "action", decision.action.clone())
            .await;
        debug!(chat_id = chat.0, decision = ?decision, "Decision resolved");

        actions::execute(&bot, chat, &self, decision).await;
    }
}

impl CurrentSession {
    /// 将完整会话状态压缩为 Intent 层所需的输入路由模式。
    fn input_mode(&self) -> InputMode {
        if matches!(&self.state, Some(SessionState::WaitingWorkspace { .. })) {
            InputMode::WaitingWorkspace
        } else if matches!(&self.cli, CliState::Active { .. }) {
            InputMode::ActiveCli
        } else {
            InputMode::Idle
        }
    }
}

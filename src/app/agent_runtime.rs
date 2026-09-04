use crate::{
    agent::{
        intent::{self},
        AgentKind, Resolver, TerminalKind, TerminalTool, WorkspaceSearchTool,
    },
    domain::{
        AgentSessionState as SessionState, ContextEvent, PendingLaunch, PendingShellCommand,
        Session,
    },
    infrastructure::{
        config::RouterConfig,
        persistence::{context_store::ContextStore, session_store::SessionStore},
        TerminalManager,
    },
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use teloxide::types::ChatId;
use tokio::sync::Mutex;

/// Telegram CLI 会话的业务编排者：识别输入、读取状态并分发动作。
pub struct Agent {
    pub(crate) terminals: TerminalManager,
    pending: Mutex<HashMap<i64, PendingLaunch>>,
    /// 当前等待用户确认的 Shell 命令；与 SessionState 同步写入磁盘。
    pending_shell: Mutex<HashMap<i64, PendingShellCommand>>,
    /// 同一 Telegram chat 的消息必须按顺序执行，避免“查看目录”尚未完成时“确认”抢先到达。
    pub(crate) chat_locks: Mutex<HashMap<i64, Arc<Mutex<()>>>>,
    selected: Mutex<HashMap<i64, AgentKind>>,
    pub(crate) workspace: WorkspaceSearchTool,
    pub(crate) terminal: TerminalTool,
    router: Arc<RouterConfig>,
    sessions: SessionStore,
    context: ContextStore,
    pub(super) resolver: Resolver,
    pub(super) intent: intent::Recognizer,
}

/// 当前 chat 正在使用的 Agent Session 运行时视图。
///
/// 它把持久化的 SessionState 与实际可用的 tmux 终端合并，供一次消息处理复用。
#[derive(Debug, Clone)]
pub(crate) struct CurrentSession {
    pub(crate) id: Option<String>,
    pub(crate) state: Option<SessionState>,
    pub(crate) terminal: TerminalState,
}

/// 当前 Session 与 Terminal 的关系：未选择 CLI、正在等待目录，或已连接终端。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TerminalState {
    None,
    Selected(AgentKind),
    Active {
        kind: TerminalKind,
        terminal_id: String,
    },
}

impl Agent {
    pub(crate) fn router(&self) -> &RouterConfig {
        &self.router
    }

    pub(crate) async fn selected_cli(&self, chat: ChatId) -> Option<AgentKind> {
        self.selected.lock().await.get(&chat.0).copied()
    }

    pub(crate) async fn select_cli(&self, chat: ChatId, kind: AgentKind) {
        self.selected.lock().await.insert(chat.0, kind);
    }

    pub(crate) async fn clear_selected_cli(&self, chat: ChatId) {
        self.selected.lock().await.remove(&chat.0);
    }

    pub(crate) async fn pending_workspace(&self, chat: ChatId) -> Option<PendingLaunch> {
        self.pending.lock().await.get(&chat.0).cloned()
    }

    pub(crate) async fn set_pending_workspace(&self, chat: ChatId, pending: PendingLaunch) {
        self.pending.lock().await.insert(chat.0, pending);
    }

    pub(crate) async fn take_pending_workspace(&self, chat: ChatId) -> Option<PendingLaunch> {
        self.pending.lock().await.remove(&chat.0)
    }

    pub(crate) async fn clear_pending_workspace(&self, chat: ChatId) {
        self.pending.lock().await.remove(&chat.0);
    }

    pub(crate) async fn pending_shell_command(&self, chat: ChatId) -> Option<PendingShellCommand> {
        self.pending_shell.lock().await.get(&chat.0).cloned()
    }

    pub(crate) async fn take_pending_shell_command(
        &self,
        chat: ChatId,
    ) -> Option<PendingShellCommand> {
        self.pending_shell.lock().await.remove(&chat.0)
    }

    pub(crate) async fn set_pending_shell_command(
        &self,
        chat: ChatId,
        command: PendingShellCommand,
    ) {
        self.pending_shell.lock().await.insert(chat.0, command);
    }

    pub async fn new(
        home: PathBuf,
        router: Arc<RouterConfig>,
        catalog: Arc<intent::IntentCatalog>,
    ) -> anyhow::Result<Self> {
        let terminals = TerminalManager::new(home.clone());
        terminals.restore().await;
        let sessions = SessionStore::new(&home);
        let context = ContextStore::new(&home);
        let restored = sessions.restore().await;
        let mut selected = HashMap::new();
        let mut pending = HashMap::new();
        let mut pending_shell = HashMap::new();
        for (chat, session) in restored {
            terminals.migrate_legacy_chat(chat, &session.id).await;
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
                SessionState::ActiveTerminal { .. } => {}
                SessionState::WaitingShellCommand {
                    terminal_session_id,
                    command,
                    description,
                } => {
                    pending_shell.insert(
                        chat,
                        PendingShellCommand {
                            terminal_session_id,
                            command,
                            description,
                        },
                    );
                }
            }
        }
        let resolver = Resolver::new(&router, Arc::clone(&catalog))?;
        Ok(Self {
            terminals,
            pending: Mutex::new(pending),
            pending_shell: Mutex::new(pending_shell),
            chat_locks: Mutex::new(HashMap::new()),
            selected: Mutex::new(selected),
            workspace: WorkspaceSearchTool::new(home),
            terminal: TerminalTool,
            router: Arc::clone(&router),
            sessions,
            context,
            resolver,
            intent: intent::Recognizer::new(router, catalog),
        })
    }

    pub(crate) async fn persist_agent_session(&self, chat: ChatId) {
        let session = self.sessions.ensure_current(chat.0).await;
        let selected = self.selected.lock().await.get(&chat.0).copied();
        let pending = self.pending.lock().await.get(&chat.0).cloned();
        let pending_shell = self.pending_shell.lock().await.get(&chat.0).cloned();
        let terminal = self
            .terminals
            .active(&session.id)
            .await
            .map(|terminal| terminal.id);
        self.sessions
            .save_state(chat.0, selected, pending, pending_shell, terminal)
            .await;
    }

    pub(crate) async fn new_session(&self, chat: ChatId) {
        self.sessions.new_session(chat.0).await;
    }

    /// 调试重置：清空当前 chat 的 Session、上下文和受管理的 tmux 终端。
    pub(crate) async fn reset_chat(&self, chat: ChatId) {
        self.pending.lock().await.remove(&chat.0);
        self.pending_shell.lock().await.remove(&chat.0);
        self.selected.lock().await.remove(&chat.0);
        self.terminals.reset_chat(chat.0).await;
        self.sessions.clear_chat(chat.0).await;
        self.context.clear_chat(chat.0);
        self.sessions.new_session(chat.0).await;
    }

    /// 关闭当前 Agent Session，并清除仅属于该流程的内存状态。
    pub(crate) async fn close_session(&self, chat: ChatId) -> Option<Session> {
        let session = self.sessions.close_current(chat.0).await?;
        self.pending.lock().await.remove(&chat.0);
        self.pending_shell.lock().await.remove(&chat.0);
        self.selected.lock().await.remove(&chat.0);
        self.terminals.clear_current(&session.id).await;
        // Agent 对每个 chat 始终保留一个当前 Session；关闭旧会话后立即补一个 Initial 会话。
        self.sessions.new_session(chat.0).await;
        Some(session)
    }

    pub(crate) async fn record_context(
        &self,
        chat: ChatId,
        role: &str,
        content: impl Into<String>,
    ) {
        let session_id = self.sessions.ensure_current(chat.0).await.id;
        self.context.record(session_id, chat.0, role, content);
    }

    /// 当前 Agent Session 的有限历史，用于 Agent 聊天和 Shell 命令建议。
    pub(crate) async fn recent_context(&self, chat: ChatId) -> Vec<ContextEvent> {
        let Some(session) = self.sessions.current(chat.0).await else {
            return vec![];
        };
        self.context.recent(&session.id, chat.0, 12)
    }

    pub(crate) async fn list_sessions(&self, chat: ChatId) -> (Vec<Session>, Option<String>) {
        self.sessions.list(chat.0).await
    }

    /// 返回当前 Agent Session ID，供 TerminalManager 把终端正确归属到该会话。
    pub(crate) async fn current_session_id(&self, chat: ChatId) -> String {
        self.sessions.ensure_current(chat.0).await.id
    }

    /// 一次性解析当前 Agent Session 与其关联 CLI 是否真的仍在运行。
    pub(crate) async fn get_current_session(&self, chat: ChatId) -> CurrentSession {
        let Some(session) = self.sessions.current(chat.0).await else {
            return CurrentSession {
                id: None,
                state: None,
                terminal: TerminalState::None,
            };
        };
        let cli = match &session.state {
            SessionState::Initial => TerminalState::None,
            SessionState::CliSelected { kind } => TerminalState::Selected(*kind),
            SessionState::WaitingWorkspace { kind, .. } => kind
                .map(TerminalState::Selected)
                .unwrap_or(TerminalState::None),
            SessionState::WaitingShellCommand {
                terminal_session_id,
                ..
            } => match self.terminals.active(&session.id).await {
                Some(active) if active.id == *terminal_session_id => TerminalState::Active {
                    kind: active.kind,
                    terminal_id: terminal_session_id.clone(),
                },
                _ => TerminalState::None,
            },
            SessionState::ActiveTerminal {
                terminal_session_id,
            } => match self.terminals.active(&session.id).await {
                Some(active) if active.id == *terminal_session_id => TerminalState::Active {
                    kind: active.kind,
                    terminal_id: terminal_session_id.clone(),
                },
                _ => TerminalState::None,
            },
        };
        CurrentSession {
            id: Some(session.id),
            state: Some(session.state),
            terminal: cli,
        }
    }

    /// 切换当前 Agent Session，并把持久化状态恢复到运行时内存。
    pub(crate) async fn switch_session(&self, chat: ChatId, id: &str) -> Result<Session, String> {
        let session = self
            .sessions
            .select(chat.0, id)
            .await
            .ok_or_else(|| "未找到可切换的 Agent 会话。".to_owned())?;
        self.pending.lock().await.remove(&chat.0);
        self.pending_shell.lock().await.remove(&chat.0);
        self.selected.lock().await.remove(&chat.0);
        self.terminals.clear_current(&session.id).await;
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
                terminal_session_id,
            } => {
                if !self
                    .terminals
                    .select(&session.id, terminal_session_id)
                    .await
                {
                    return Err("该会话对应的 tmux 终端已结束或不可用。".to_owned());
                }
            }
            SessionState::WaitingShellCommand {
                terminal_session_id,
                command,
                description,
            } => {
                if !self
                    .terminals
                    .select(&session.id, terminal_session_id)
                    .await
                {
                    return Err("该会话对应的 tmux 终端已结束或不可用。".to_owned());
                }
                self.pending_shell.lock().await.insert(
                    chat.0,
                    PendingShellCommand {
                        terminal_session_id: terminal_session_id.clone(),
                        command: command.clone(),
                        description: description.clone(),
                    },
                );
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

    /// 接口层调用的单消息入口；一轮用户交互的编排实现在 app::interaction。
    pub async fn run(
        self: Arc<Self>,
        bot: teloxide::Bot,
        output: Arc<dyn crate::service::output::ChatOutput>,
        chat: ChatId,
        text: &str,
    ) {
        crate::app::interaction::run(self, bot, output, chat, text).await;
    }
}

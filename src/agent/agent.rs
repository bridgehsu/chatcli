use crate::{
    agent::{actions, intent, AgentKind, Decision, Resolver},
    infrastructure::config::RouterConfig,
    session::TerminalManager,
    tools::{TerminalTool, WorkspaceSearchTool},
};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use teloxide::{prelude::*, types::ChatId};
use tokio::sync::Mutex;

/// Telegram CLI 会话的业务编排者：识别输入、读取状态并分发动作。
pub struct Agent {
    pub(super) terminals: TerminalManager,
    pub(super) pending: Mutex<HashMap<i64, PendingLaunch>>,
    pub(super) selected: Mutex<HashMap<i64, AgentKind>>,
    pub(super) workspace: WorkspaceSearchTool,
    pub(super) terminal: TerminalTool,
    resolver: Resolver,
    intent: intent::Recognizer,
}

pub(super) struct PendingLaunch {
    pub(super) kind: AgentKind,
    pub(super) candidates: Vec<PathBuf>,
}

impl Agent {
    pub async fn new(home: PathBuf, router: Arc<RouterConfig>) -> anyhow::Result<Self> {
        let terminals = TerminalManager::new(home.clone());
        terminals.restore().await;
        let resolver = Resolver::new(&router)?;
        Ok(Self {
            terminals,
            pending: Mutex::new(HashMap::new()),
            selected: Mutex::new(HashMap::new()),
            workspace: WorkspaceSearchTool::new(home),
            terminal: TerminalTool,
            resolver,
            intent: intent::Recognizer::new(router),
        })
    }

    pub async fn run(self: Arc<Self>, bot: Bot, chat: ChatId, text: &str) {
        // 所有输入（按钮、命令与自然语言）统一先经过意图层。
        let intent = self.intent.run(text).await;
        let decision = self.resolver.decide(intent);

        // 目录相关意图需要优先执行，即使当前已有活动终端。
        if !matches!(
            decision,
            Decision::SelectWorkspace | Decision::ListDirectories
        ) && self.handle_session_state(&bot, chat, text).await
        {
            return;
        }

        actions::execute(&bot, chat, &self, decision).await;
    }

    /// 处理依赖会话状态的普通文本。返回 true 代表消息已被消费。
    async fn handle_session_state(self: &Arc<Self>, bot: &Bot, chat: ChatId, text: &str) -> bool {
        if self.pending.lock().await.contains_key(&chat.0) {
            actions::workspace::launch(bot, chat, Arc::clone(self), text).await;
            return true;
        }
        if self.terminals.active(chat.0).await.is_some() {
            actions::session::forward(bot, chat, self, text.to_owned()).await;
            return true;
        }
        false
    }
}

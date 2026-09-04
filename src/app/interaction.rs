//! 一轮用户交互的应用主流程。
//!
//! 这里负责串行化、上下文、意图路由和动作执行；不包含 Telegram transport、
//! tmux 命令或 JSON 持久化的具体细节。

use crate::{
    agent::{intent, intent::IntentRoute, Decision},
    app::{action_executor, Agent},
    service::{output::ChatOutput, shell_service, terminal_service, workspace_service},
};
use std::sync::Arc;
use teloxide::{prelude::*, types::ChatId};
use tracing::{debug, info};

pub async fn run(
    agent: Arc<Agent>,
    bot: Bot,
    output: Arc<dyn ChatOutput>,
    chat: ChatId,
    text: &str,
) {
    let chat_lock = {
        let mut locks = agent.chat_locks.lock().await;
        Arc::clone(
            locks
                .entry(chat.0)
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
        )
    };
    let _guard = chat_lock.lock().await;
    let current = agent.get_current_session(chat).await;
    info!(
        chat_id = chat.0,
        session_id = ?current.id,
        terminal = ?current.terminal,
        "Resolved current Agent Session"
    );
    agent.record_context(chat, "user", text).await;
    let intent = agent.intent.run(text, &current).await;
    dispatch_intent(&agent, &bot, output, chat, intent).await;
}

async fn dispatch_intent(
    agent: &Arc<Agent>,
    bot: &Bot,
    output: Arc<dyn ChatOutput>,
    chat: ChatId,
    intent: intent::Intent,
) {
    match intent.route {
        IntentRoute::Action => execute_intent(agent, bot, output, chat, intent).await,
        IntentRoute::WorkspaceInput => {
            workspace_service::launch(
                bot,
                chat,
                Arc::clone(agent),
                intent.argument.as_deref().unwrap_or_default(),
            )
            .await;
        }
        IntentRoute::CliInput => {
            terminal_service::forward(bot, chat, agent, intent.argument.unwrap_or_default()).await;
        }
        IntentRoute::ShellRequest => {
            shell_service::suggest(bot, chat, agent, intent.argument.unwrap_or_default()).await;
        }
    }
}

async fn execute_intent(
    agent: &Arc<Agent>,
    bot: &Bot,
    output: Arc<dyn ChatOutput>,
    chat: ChatId,
    intent: intent::Intent,
) {
    agent
        .record_context(chat, "intent", intent.name.clone())
        .await;
    info!(chat_id = chat.0, intent = %intent.name, cli = ?intent.cli, "Intent recognized");
    let decision: Decision = agent.resolver.decide(intent);
    agent
        .record_context(chat, "action", decision.action.clone())
        .await;
    debug!(chat_id = chat.0, decision = ?decision, "Decision resolved");
    action_executor::execute(bot, output, chat, agent, decision).await;
}

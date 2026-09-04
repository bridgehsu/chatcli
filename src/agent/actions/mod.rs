//! Agent 的工具注册表：将 YAML 的 `action` 字段分发到已实现的业务工具。

pub mod cli;
pub mod session;
pub mod workspace;

use crate::agent::{intent::commands::Command, Agent, Decision};
use std::sync::Arc;
use teloxide::{prelude::*, types::ChatId};
use tracing::{info, warn};

/// 所有配置动作的唯一执行入口。
pub async fn execute(bot: &Bot, chat: ChatId, state: &Arc<Agent>, decision: Decision) {
    info!(chat_id = chat.0, action = %decision.action, "Executing agent action");
    match decision.action.as_str() {
        "cli.offer" => {
            cli::initial(bot, chat, state, "请先选择要使用的 CLI：Codex 或 Cursor。").await
        }
        "system.unknown" => {
            cli::initial(
                bot,
                chat,
                state,
                "暂时无法识别你的需求，请选择 Codex 或 Cursor。",
            )
            .await
        }
        "manager.new" => session::new_session(bot, chat, state).await,
        "manager.close" => session::close(bot, chat, state).await,
        "manager.list" => session::list_sessions(bot, chat, state, false).await,
        "manager.switch" => session::list_sessions(bot, chat, state, true).await,
        "manager.switch_id" => match decision.argument.as_deref() {
            Some(id) => session::switch_agent_session(bot, chat, state, id).await,
            None => {
                let _ = bot
                    .send_message(chat, "请提供要切换的 Agent 会话 ID。")
                    .await;
            }
        },
        "manager.reset" => session::reset(bot, chat, state).await,
        "agent.introduce" => introduce(bot, chat, state).await,
        "workspace.select" => workspace::select_workspace(bot, chat, state).await,
        "workspace.list" => workspace::list_directories(bot, chat, state).await,
        "terminal.close" => execute_command(bot, chat, state, Command::Close).await,
        "cli.choose" => match decision.cli {
            Some(kind) => execute_command(bot, chat, state, Command::NewSession(kind)).await,
            None => cli::initial(bot, chat, state, "请选择 Codex 或 Cursor。").await,
        },
        unknown => {
            warn!(
                chat_id = chat.0,
                action = unknown,
                "No tool registered for configured action"
            );
            cli::initial(bot, chat, state, "该功能尚未接入，请选择 Codex 或 Cursor。").await;
        }
    }
}

async fn introduce(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    let message = "我是 ChatCLI：通过 Telegram 管理本机 CodeX 和 Cursor 的 Agent。\n\n我可以创建和切换 Agent 会话、选择工作目录、启动或连接 tmux CLI 会话，并把普通消息转发给当前 CLI。\n\n你可以直接发送：CodeX、Cursor、新建会话、会话列表、切换会话、获取当前目录下的目录或 /reset。";
    state.record_context(chat, "agent", message).await;
    let _ = bot.send_message(chat, message).await;
}

/// Executes deterministic reply-keyboard and slash commands.
pub(crate) async fn execute_command(bot: &Bot, chat: ChatId, state: &Arc<Agent>, command: Command) {
    match command {
        Command::Help => {
            cli::initial(bot, chat, state, "请选择要使用的 CLI：CodeX 或 Cursor。").await
        }
        Command::CliHome(kind) => cli::home(bot, chat, state, kind).await,
        Command::NewSession(kind) => cli::begin(bot, chat, state, kind).await,
        Command::List(kind) => session::list(bot, chat, state, kind).await,
        Command::SelectSession(id) => session::select(bot, chat, state, &id).await,
        Command::Reset => session::reset(bot, chat, state).await,
        Command::Cancel | Command::Back => {
            state.pending.lock().await.remove(&chat.0);
            state.persist_agent_session(chat).await;
            let selected = { state.selected.lock().await.get(&chat.0).copied() };
            if let Some(kind) = selected {
                cli::home(bot, chat, state, kind).await
            } else {
                cli::initial(bot, chat, state, "已取消。").await
            }
        }
        Command::Screen => session::screen(bot, chat, state).await,
        Command::Attach => session::attach(bot, chat, state).await,
        Command::Stop => session::stop(bot, chat, state).await,
        Command::Close => session::close(bot, chat, state).await,
        Command::Status => session::status(bot, chat, state).await,
        Command::Text(_) => unreachable!("文本不应生成 Control 命令"),
    }
}

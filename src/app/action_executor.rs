//! Agent 的工具注册表：将 YAML 的 `action` 字段分发到已实现的业务工具。

use crate::{
    agent::Decision,
    app::Agent,
    service::{
        chat_service as chat, cli_service as cli, output::ChatOutput, session_service as session,
        shell_service as shell, system_service as system, terminal_service as terminal,
        workspace_service as workspace,
    },
};
use std::sync::Arc;
use teloxide::{prelude::*, types::ChatId};
use tracing::{info, warn};

/// 所有配置动作的唯一执行入口。
pub async fn execute(
    bot: &Bot,
    output: Arc<dyn ChatOutput>,
    chat: ChatId,
    state: &Arc<Agent>,
    decision: Decision,
) {
    info!(chat_id = chat.0, action = %decision.action, "Executing agent action");
    match decision.action.as_str() {
        "cli.offer" => {
            cli::initial(bot, chat, state, "请先选择要使用的 CLI：Codex 或 Cursor。").await
        }
        "terminal.open" => terminal::open_shell(bot, chat, state).await,
        "shell.confirm" => shell::confirm(bot, chat, state).await,
        "shell.cancel" => shell::cancel(bot, chat, state).await,
        "shell.edit" => match decision.argument.as_deref() {
            Some(command) => shell::edit(bot, chat, state, command).await,
            None => {
                let _ = bot
                    .send_message(chat, "请在“修改为：”后提供一行 Shell 命令。")
                    .await;
            }
        },
        "shell.waiting_confirmation" => shell::waiting_confirmation(bot, chat, state).await,
        "system.unknown" | "agent.chat" => {
            chat::reply(&*output, chat, state, decision.argument).await
        }
        "session.create" => session::new_session(&*output, chat, state).await,
        "session.close" => session::close(&*output, chat, state).await,
        "session.list" => session::list(&*output, chat, state, false).await,
        "session.switch" => session::list(&*output, chat, state, true).await,
        "session.switch_id" => match decision.argument.as_deref() {
            Some(id) => session::switch(&*output, chat, state, id).await,
            None => {
                let _ = bot
                    .send_message(chat, "请提供要切换的 Agent 会话 ID。")
                    .await;
            }
        },
        "session.reset" => system::reset(&*output, chat, state).await,
        "agent.introduce" => system::introduce(&*output, chat, state).await,
        "workspace.select" => workspace::select_workspace(bot, chat, state).await,
        "workspace.list" => workspace::list_directories(bot, chat, state).await,
        "workspace.cancel" => workspace::cancel(bot, chat, state).await,
        "system.help" => system::help(&*output, chat, state).await,
        "terminal.close" => terminal::close(bot, chat, state).await,
        "terminal.list" => terminal::list(bot, chat, state, false).await,
        "terminal.switch" => terminal::list(bot, chat, state, true).await,
        "terminal.switch_id" => match decision.argument.as_deref() {
            Some(id) => terminal::switch(bot, chat, state, id).await,
            None => {
                let _ = bot.send_message(chat, "请提供要切换的终端 ID。").await;
            }
        },
        "cli.choose" => match decision.cli {
            Some(kind) => cli::begin(bot, chat, state, kind).await,
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

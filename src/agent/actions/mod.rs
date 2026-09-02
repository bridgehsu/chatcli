//! Agent 的业务动作层：负责状态更新、工具调用与 Telegram 回复。

pub mod cli;
pub mod session;
pub mod workspace;

use crate::{
    agent::intent::commands::Command,
    agent::{Agent, Decision},
};
use std::sync::Arc;
use teloxide::{prelude::*, types::ChatId};

/// 执行 Agent 已经确定的下一步动作。
pub async fn execute(bot: &Bot, chat: ChatId, state: &Arc<Agent>, decision: Decision) {
    match decision {
        Decision::OfferCli => {
            cli::initial(bot, chat, state, "请先选择要使用的 CLI：Codex 或 Cursor。").await
        }
        Decision::Unknown => {
            cli::initial(
                bot,
                chat,
                state,
                "暂时无法识别你的需求，请选择 Codex 或 Cursor。",
            )
            .await
        }
        Decision::SelectWorkspace => workspace::select_workspace(bot, chat, state).await,
        Decision::ListDirectories => workspace::list_directories(bot, chat, state).await,
        Decision::Control(command) => control(bot, chat, state, command).await,
    }
}

async fn control(bot: &Bot, chat: ChatId, state: &Arc<Agent>, command: Command) {
    match command {
        Command::Help => {
            cli::initial(bot, chat, state, "请选择要使用的 CLI：Codex 或 Cursor。").await
        }
        Command::CliHome(kind) => cli::home(bot, chat, state, kind).await,
        Command::NewSession(kind) => cli::begin(bot, chat, state, kind).await,
        Command::List(kind) => session::list(bot, chat, state, kind).await,
        Command::SelectSession(id) => session::select(bot, chat, state, &id).await,
        Command::Reset => session::reset(bot, chat, state).await,
        Command::Cancel | Command::Back => {
            state.pending.lock().await.remove(&chat.0);
            if let Some(kind) = state.selected.lock().await.get(&chat.0).copied() {
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
        Command::Text(_) => unreachable!("文本不应生成 Control 决策"),
    }
}

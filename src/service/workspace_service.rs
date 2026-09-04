use crate::{
    agent::SearchResult, app::Agent, domain::PendingLaunch,
    interfaces::channels::telegram::presenter, service::cli_service,
};
use std::{path::PathBuf, sync::Arc};
use teloxide::{
    prelude::*,
    types::{ChatId, ParseMode},
};
use tracing::info;

pub async fn select_workspace(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    let selected = state.selected_cli(chat).await;
    match selected {
        Some(kind) => cli_service::begin(bot, chat, state, kind).await,
        None => {
            cli_service::initial(bot, chat, state, "请先选择要使用的 CLI：Codex 或 Cursor。").await
        }
    }
}

pub async fn list_directories(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    match state.workspace.list_directories().await {
        Ok(directories) if directories.is_empty() => {
            let _ = bot
                .send_message(chat, "Home 目录下没有可显示的工作目录。")
                .await;
        }
        Ok(directories) => {
            let selected = state.selected_cli(chat).await;
            if let Some(kind) = selected {
                state
                    .set_pending_workspace(
                        chat,
                        PendingLaunch {
                            kind: Some(kind),
                            candidates: directories.clone(),
                            workspace: None,
                        },
                    )
                    .await;
                state.persist_agent_session(chat).await;
                candidates_message(bot, chat, "请选择工作目录，回复序号", &directories).await;
            } else {
                state
                    .set_pending_workspace(
                        chat,
                        PendingLaunch {
                            kind: None,
                            candidates: directories.clone(),
                            workspace: None,
                        },
                    )
                    .await;
                state.persist_agent_session(chat).await;
                candidates_message(bot, chat, "请选择工作目录，回复序号", &directories).await;
            }
        }
        Err(error) => {
            let _ = bot
                .send_message(chat, format!("读取工作目录失败：{error}"))
                .await;
        }
    }
}

pub async fn launch(bot: &Bot, chat: ChatId, state: Arc<Agent>, input: &str) {
    let Some(pending) = state.take_pending_workspace(chat).await else {
        return;
    };
    info!(
        chat_id = chat.0,
        input_len = input.chars().count(),
        "Resolving selected workspace"
    );
    state.persist_agent_session(chat).await;
    let result = if pending.candidates.is_empty() {
        state.workspace.resolve_workspace(input).await
    } else {
        input
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|i| pending.candidates.get(i.saturating_sub(1)).cloned())
            .map(SearchResult::Unique)
            .ok_or_else(|| "请选择候选目录的序号。".to_owned())
    };
    let workspace = match result {
        Ok(SearchResult::Unique(path)) => path,
        Ok(SearchResult::Multiple(candidates)) => {
            state
                .set_pending_workspace(
                    chat,
                    PendingLaunch {
                        kind: pending.kind,
                        candidates: candidates.clone(),
                        workspace: None,
                    },
                )
                .await;
            state.persist_agent_session(chat).await;
            candidates_message(bot, chat, "找到多个目录，请回复序号", &candidates).await;
            return;
        }
        Ok(SearchResult::NotFound) | Err(_) => {
            state
                .set_pending_workspace(
                    chat,
                    PendingLaunch {
                        kind: pending.kind,
                        candidates: vec![],
                        workspace: None,
                    },
                )
                .await;
            state.persist_agent_session(chat).await;
            let _ = bot
                .send_message(chat, "未找到对应目录，请重新发送目录路径或目录名。")
                .reply_markup(presenter::create_keyboard())
                .await;
            return;
        }
    };
    match pending.kind {
        Some(kind) => cli_service::open(bot, chat, state, kind, workspace).await,
        None => {
            state
                .set_pending_workspace(
                    chat,
                    PendingLaunch {
                        kind: None,
                        candidates: vec![],
                        workspace: Some(workspace.clone()),
                    },
                )
                .await;
            state.persist_agent_session(chat).await;
            cli_service::initial(
                bot,
                chat,
                &state,
                &format!(
                    "已选择工作目录 {}，请选择要启动的 CLI。",
                    workspace.display()
                ),
            )
            .await;
        }
    }
}

/// 取消目录选择，恢复到没有待处理输入的 Session 状态。
pub async fn cancel(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    state.clear_pending_workspace(chat).await;
    state.persist_agent_session(chat).await;
    let _ = bot.send_message(chat, "已取消目录选择。").await;
}

async fn candidates_message(bot: &Bot, chat: ChatId, title: &str, paths: &[PathBuf]) {
    let lines = paths
        .iter()
        .take(20)
        .enumerate()
        .map(|(i, p)| {
            // 候选路径实际保存在 pending 中；Telegram 只展示末级目录名，避免重复 Home 前缀。
            let name = p
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
                .unwrap_or_else(|| p.to_string_lossy().into_owned());
            format!("{}. <code>{}</code>", i + 1, presenter::html_escape(&name))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let _ = bot
        .send_message(chat, format!("{}：\n{}", title, lines))
        .parse_mode(ParseMode::Html)
        .reply_markup(presenter::create_keyboard())
        .await;
}

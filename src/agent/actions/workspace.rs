use crate::{
    agent::{actions::cli, agent::PendingLaunch, Agent},
    channels::telegram::presenter,
    tools::SearchResult,
};
use std::{path::PathBuf, sync::Arc};
use teloxide::{
    prelude::*,
    types::{ChatId, ParseMode},
};

pub async fn select_workspace(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    match state.selected.lock().await.get(&chat.0).copied() {
        Some(kind) => cli::begin(bot, chat, state, kind).await,
        None => cli::initial(bot, chat, state, "请先选择要使用的 CLI：Codex 或 Cursor。").await,
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
            if let Some(kind) = state.selected.lock().await.get(&chat.0).copied() {
                state.pending.lock().await.insert(
                    chat.0,
                    PendingLaunch {
                        kind,
                        candidates: directories.clone(),
                    },
                );
                candidates_message(bot, chat, "请选择工作目录，回复序号", &directories).await;
            } else {
                candidates_message(bot, chat, "Home 下可选目录", &directories).await;
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
    let Some(pending) = state.pending.lock().await.remove(&chat.0) else {
        return;
    };
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
            state.pending.lock().await.insert(
                chat.0,
                PendingLaunch {
                    kind: pending.kind,
                    candidates: candidates.clone(),
                },
            );
            candidates_message(bot, chat, "找到多个目录，请回复序号", &candidates).await;
            return;
        }
        Ok(SearchResult::NotFound) | Err(_) => {
            state.pending.lock().await.insert(
                chat.0,
                PendingLaunch {
                    kind: pending.kind,
                    candidates: vec![],
                },
            );
            let _ = bot
                .send_message(chat, "未找到对应目录，请重新发送目录路径或目录名。")
                .reply_markup(presenter::create_keyboard())
                .await;
            return;
        }
    };
    cli::open(bot, chat, state, pending.kind, workspace).await;
}

async fn candidates_message(bot: &Bot, chat: ChatId, title: &str, paths: &[PathBuf]) {
    let lines = paths
        .iter()
        .take(20)
        .enumerate()
        .map(|(i, p)| {
            format!(
                "{}. <code>{}</code>",
                i + 1,
                presenter::html_escape(&p.display().to_string())
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let _ = bot
        .send_message(chat, format!("{}：\n{}", title, lines))
        .parse_mode(ParseMode::Html)
        .reply_markup(presenter::create_keyboard())
        .await;
}

//! Shell 自然语言操作：先生成候选命令，再由用户确认后执行。

use crate::{
    agent::{policy::shell_policy, TerminalKind},
    app::Agent,
    domain::PendingShellCommand,
    infrastructure::llm_client,
    interfaces::channels::telegram::presenter,
    service::terminal_service,
};
use serde::Deserialize;
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{ChatId, ParseMode},
};
use tracing::{info, warn};

#[derive(Deserialize)]
struct CommandProposal {
    command: String,
    description: String,
}

/// 为自然语言请求生成一条 Shell 候选命令，但绝不在此处执行。
pub async fn suggest(bot: &Bot, chat: ChatId, state: &Arc<Agent>, request: String) {
    let session_id = state.current_session_id(chat).await;
    let Some(active) = state.terminals.active(&session_id).await else {
        let _ = bot.send_message(chat, "当前没有选中的 Shell 终端。").await;
        return;
    };
    if active.kind != TerminalKind::Shell {
        let _ = bot
            .send_message(chat, "当前不是 Shell 终端，普通消息会直接发送到当前 CLI。")
            .await;
        return;
    }

    let proposal = match generate_command(state, chat, &request).await {
        Ok(proposal) => proposal,
        Err(message) => {
            let _ = bot.send_message(chat, message).await;
            return;
        }
    };
    let pending = PendingShellCommand {
        terminal_session_id: active.id,
        command: proposal.command,
        description: proposal.description,
    };
    save_pending(state, chat, pending.clone()).await;
    state
        .record_context(chat, "shell_proposal", pending.command.clone())
        .await;
    info!(
        chat_id = chat.0,
        "Shell command proposal is waiting for confirmation"
    );

    show_proposal(bot, chat, &pending).await;
}

/// 用户确认后，才将候选命令写入此前绑定的 Shell tmux 终端。
pub async fn confirm(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    let Some(pending) = state.take_pending_shell_command(chat).await else {
        let _ = bot
            .send_message(chat, "当前没有等待确认的 Shell 命令。")
            .await;
        return;
    };
    let session_id = state.current_session_id(chat).await;
    let valid_target = state
        .terminals
        .active(&session_id)
        .await
        .map(|terminal| {
            terminal.id == pending.terminal_session_id && terminal.kind == TerminalKind::Shell
        })
        .unwrap_or(false);
    if !valid_target {
        state.persist_agent_session(chat).await;
        let _ = bot
            .send_message(chat, "目标 Shell 终端已切换或不可用，命令没有执行。")
            .await;
        return;
    }
    state.persist_agent_session(chat).await;
    state
        .record_context(chat, "shell_execute", pending.command.clone())
        .await;
    terminal_service::forward(bot, chat, state, pending.command).await;
}

/// 放弃候选命令，并恢复当前终端的正常输入状态。
pub async fn cancel(bot: &Bot, chat: ChatId, state: &Arc<Agent>) {
    if state.take_pending_shell_command(chat).await.is_some() {
        state.persist_agent_session(chat).await;
        let _ = bot
            .send_message(chat, "已取消 Shell 命令，不会执行任何操作。")
            .await;
    } else {
        let _ = bot
            .send_message(chat, "当前没有等待确认的 Shell 命令。")
            .await;
    }
}

/// 用户手动修改候选命令；仍使用和模型输出相同的安全限制，并要求再次确认。
pub async fn edit(bot: &Bot, chat: ChatId, state: &Arc<Agent>, command: &str) {
    let Some(mut pending) = state.pending_shell_command(chat).await else {
        let _ = bot
            .send_message(chat, "当前没有可修改的 Shell 候选命令。")
            .await;
        return;
    };
    if let Err(message) = shell_policy::validate(command) {
        let _ = bot.send_message(chat, message).await;
        return;
    }
    pending.command = command.trim().to_owned();
    pending.description = "用户修改后的命令。".to_owned();
    save_pending(state, chat, pending.clone()).await;
    state
        .record_context(chat, "shell_proposal", pending.command.clone())
        .await;
    show_proposal(bot, chat, &pending).await;
}

pub async fn waiting_confirmation(bot: &Bot, chat: ChatId, _state: &Arc<Agent>) {
    let _ = bot
        .send_message(
            chat,
            "当前命令正在等待确认。请回复“确认”执行、回复“取消”放弃，或发送“修改为：命令”后再次确认。",
        )
        .await;
}

async fn generate_command(
    state: &Agent,
    chat: ChatId,
    request: &str,
) -> Result<CommandProposal, String> {
    let router = state.router();
    if !router.enabled || router.api_key.is_empty() {
        return Err(
            "未配置模型，无法把自然语言转换为 Shell 命令。可使用 `$ 命令` 直接发送。".to_owned(),
        );
    }
    let history = state.recent_context(chat).await;
    let mut messages = vec![serde_json::json!({
        "role": "system",
        "content": "你是 macOS zsh 的只读 Shell 命令建议器。根据用户中文请求只返回 JSON：{\"command\":\"一行 Shell 命令\",\"description\":\"简短中文说明\"}。不要使用 Markdown。只允许生成 ls、pwd、find、rg、grep、cat、head、tail、wc、du、df、ps、git、which、echo、tree、stat 这类只读命令；其他需求 command 返回空字符串并说明原因。不得生成管道、重定向、命令替换或破坏性命令。"
    })];
    messages.extend(history.into_iter().map(|event| {
        serde_json::json!({
            "role": if event.role == "user" { "user" } else { "assistant" },
            "content": event.content,
        })
    }));
    if messages.len() == 1 {
        messages.push(serde_json::json!({"role": "user", "content": request}));
    }
    info!(model = %router.model, request = %request, "Requesting Shell command proposal");
    let text = llm_client::complete(router, 0.0, messages)
        .await
        .map_err(|error| {
            warn!(%error, "Shell command proposal request failed");
            "生成 Shell 命令失败，请稍后重试。".to_owned()
        })?;
    let output = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let proposal: CommandProposal = serde_json::from_str(output)
        .map_err(|_| "模型没有返回可用的 Shell 命令，未执行任何操作。".to_owned())?;
    shell_policy::validate(&proposal.command)?;
    Ok(proposal)
}

async fn save_pending(state: &Arc<Agent>, chat: ChatId, pending: PendingShellCommand) {
    state.set_pending_shell_command(chat, pending).await;
    state.persist_agent_session(chat).await;
}

async fn show_proposal(bot: &Bot, chat: ChatId, pending: &PendingShellCommand) {
    let _ = bot
        .send_message(
            chat,
            format!(
                "<b>将执行 Shell 命令</b>\n\n<code>{}</code>\n\n{}\n\n回复“确认”执行；回复“取消”放弃。\n发送“修改为：命令”可编辑后再次确认。\n<i>如需直接发送 Shell 命令，请以 $ 或 ! 开头。</i>",
                presenter::html_escape(&pending.command),
                presenter::html_escape(&pending.description),
            ),
        )
        .parse_mode(ParseMode::Html)
        .await;
}

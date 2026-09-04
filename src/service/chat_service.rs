//! Agent 自身的普通对话能力，不操作 tmux 或工作目录。

use crate::{app::Agent, infrastructure::llm_client, service::output::ChatOutput};
use std::sync::Arc;
use teloxide::types::ChatId;

pub async fn reply(
    output: &dyn ChatOutput,
    chat: ChatId,
    state: &Arc<Agent>,
    input: Option<String>,
) {
    let input = input.unwrap_or_else(|| "请介绍 ChatCLI 可以做什么。".to_owned());
    let router = state.router();
    if !router.enabled || router.api_key.is_empty() {
        let _ = output
            .send(
                chat.0,
                "当前没有配置聊天模型。你可以发送“打开终端”、CodeX 或 Cursor。".to_owned(),
            )
            .await;
        return;
    }
    let history = state.recent_context(chat).await;
    let mut messages = vec![serde_json::json!({
        "role": "system",
        "content": "你是 ChatCLI 的中文助手。直接、简洁地回答用户。你不能假装已经执行 Shell、tmux、CodeX 或 Cursor 操作；需要执行时请说明用户可用的 ChatCLI 指令。"
    })];
    messages.extend(history.into_iter().map(|event| {
        serde_json::json!({
            "role": if event.role == "user" { "user" } else { "assistant" },
            "content": event.content,
        })
    }));
    if messages.len() == 1 {
        messages.push(serde_json::json!({"role": "user", "content": input}));
    }
    let answer = match llm_client::complete(router, 0.4, messages).await {
        Ok(answer) => answer,
        Err(error) => {
            let _ = output
                .send(chat.0, format!("Agent 聊天失败：{error}"))
                .await;
            return;
        }
    };
    state.record_context(chat, "agent", &answer).await;
    let _ = output.send(chat.0, answer).await;
}

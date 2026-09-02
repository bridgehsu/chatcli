//! 意图识别的模型兜底层。
//!
//! 精确规则未识别时，依次尝试本地模型与远程 LLM；模型不可用、低置信度或
//! 返回内容不合法时统一返回 `None`，由上层安全降级为 `Unknown`。

use std::time::Duration;

use serde::Deserialize;

use super::{normalize::NormalizedInput, Intent, IntentKind, IntentName};
use crate::{agent::AgentKind, infrastructure::config::RouterConfig};

/// 模型结果的最低可接受置信度，避免模糊推断触发业务动作。
const MIN_CONFIDENCE: f32 = 0.60;

/// OpenAI 兼容 Chat Completions 响应的最小结构。
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
/// 单个模型候选回复。
#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}
/// 候选回复中的消息正文。
#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}
/// 约束大模型返回的意图 JSON；不接受自由文本或命令。
#[derive(Deserialize)]
struct ModelIntent {
    intent: IntentName,
    #[serde(default)]
    cli: Option<String>,
    confidence: f32,
}

pub async fn recognize(input: &NormalizedInput, router: &RouterConfig) -> Option<Intent> {
    // 本地模型优先；当前 Demo 未接入本地推理时会返回 None，继续走 LLM。
    recognize_on_device(input)
        .await
        .or(recognize_llm(input, router).await)
}

async fn recognize_on_device(_input: &NormalizedInput) -> Option<Intent> {
    // 为后续接入 Ollama、ONNX 或其他设备端分类模型预留的扩展点。
    None
}

async fn recognize_llm(input: &NormalizedInput, router: &RouterConfig) -> Option<Intent> {
    // 未显式启用路由器或没有 API Key 时，绝不发起外部网络请求。
    if !router.enabled || router.api_key.is_empty() {
        return None;
    }
    // RouterConfig 使用 OpenAI Chat Completions 兼容的基础地址。
    let endpoint = format!("{}/chat/completions", router.base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(router.timeout_secs))
        .build()
        .ok()?;
    // temperature=0 使分类结果更稳定；意图列表从 IntentName 自动生成。
    let intent_values = IntentName::classifier_values();
    let system_prompt = format!(
        "Classify the user text. Return JSON only: {{\"intent\":\"{}\",\"cli\":\"codex|cursor|null\",\"confidence\":0.0}}. Do not return shell commands.",
        intent_values,
    );
    let body = serde_json::json!({
        "model": router.model,
        "temperature": 0,
        "messages": [
            {"role":"system", "content": system_prompt},
            {"role":"user", "content": input.text}
        ]
    });
    // 任意网络、HTTP 状态或 JSON 解析错误都安全降级为 None。
    let response = client
        .post(endpoint)
        .bearer_auth(&router.api_key)
        .json(&body)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<ChatResponse>()
        .await
        .ok()?;
    parse_response(&response.choices.first()?.message.content)
}

fn parse_response(content: &str) -> Option<Intent> {
    // 兼容部分模型用 Markdown 代码块包裹 JSON 的情况。
    let content = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    // JSON 结构、置信度和枚举值均需在本地再次校验，模型结果不被直接信任。
    let output: ModelIntent = serde_json::from_str(content).ok()?;
    if !(MIN_CONFIDENCE..=1.0).contains(&output.confidence) {
        return None;
    }
    // 仅映射白名单中的意图；其余输出一律拒绝。
    let kind = match (output.intent, output.cli.as_deref()) {
        (IntentName::OpenTerminal, _) => IntentKind::OpenTerminal,
        (IntentName::SelectWorkspace, _) => IntentKind::SelectWorkspace,
        (IntentName::ListDirectories, _) => IntentKind::ListDirectories,
        (IntentName::CloseTerminal, _) => IntentKind::CloseTerminal,
        (IntentName::Unknown, _) => IntentKind::Unknown,
        (IntentName::ChooseCli, Some("codex")) => IntentKind::ChooseCli(AgentKind::Codex),
        (IntentName::ChooseCli, Some("cursor")) => IntentKind::ChooseCli(AgentKind::Cursor),
        _ => return None,
    };
    Some(Intent { kind })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_valid_choose_cli_json() {
        assert_eq!(
            parse_response(r#"{"intent":"choose_cli","cli":"codex","confidence":0.9}"#)
                .unwrap()
                .kind,
            IntentKind::ChooseCli(AgentKind::Codex)
        );
    }
    #[test]
    fn rejects_low_confidence_json() {
        assert!(parse_response(r#"{"intent":"close_terminal","confidence":0.2}"#).is_none());
    }

    #[test]
    fn builds_intent_values_from_the_enum() {
        assert_eq!(
            IntentName::classifier_values(),
            "choose_cli|open_terminal|select_workspace|list_directories|close_terminal|unknown"
        );
    }
}

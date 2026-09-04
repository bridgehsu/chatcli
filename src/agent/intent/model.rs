//! 意图识别的模型兜底层。
//!
//! 精确规则未识别时，依次尝试本地模型与远程 LLM；模型不可用、低置信度或
//! 返回内容不合法时统一返回 `None`，由上层安全降级为 `Unknown`。

use std::time::Duration;

use serde::Deserialize;
use tracing::{info, warn};

use super::{normalize::NormalizedInput, Intent, IntentCatalog};
use crate::{domain::AgentKind, infrastructure::config::RouterConfig};

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
    intent: String,
    #[serde(default)]
    cli: Option<String>,
    confidence: f32,
}

pub async fn recognize(
    input: &NormalizedInput,
    router: &RouterConfig,
    catalog: &IntentCatalog,
    allowed: &[&str],
) -> Option<Intent> {
    // 本地模型优先；当前 Demo 未接入本地推理时会返回 None，继续走 LLM。
    recognize_on_device(input)
        .await
        .or(recognize_llm(input, router, catalog, allowed).await)
}

async fn recognize_on_device(_input: &NormalizedInput) -> Option<Intent> {
    // 为后续接入 Ollama、ONNX 或其他设备端分类模型预留的扩展点。
    None
}

async fn recognize_llm(
    input: &NormalizedInput,
    router: &RouterConfig,
    catalog: &IntentCatalog,
    allowed: &[&str],
) -> Option<Intent> {
    // 未显式启用路由器或没有 API Key 时，绝不发起外部网络请求。
    if !router.enabled || router.api_key.is_empty() {
        return None;
    }
    // RouterConfig 使用 OpenAI Chat Completions 兼容的基础地址。
    let endpoint = format!("{}/chat/completions", router.base_url.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(router.timeout_secs))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            warn!(%error, "Failed to create LLM HTTP client");
            return None;
        }
    };
    // temperature=0 使分类结果更稳定；意图说明与示例来自 config/intents.yaml。
    let body = serde_json::json!({
        "model": router.model,
        "temperature": 0,
        "messages": [
            {"role":"system", "content": catalog.system_prompt_for(allowed)},
            {"role":"user", "content": input.text}
        ]
    });

    // 请求体不包含 Authorization；API Key 不会写入日志。
    info!(
        model = %router.model,
        endpoint = %endpoint,
        request_body = %body,
        "Sending LLM intent request"
    );
    let response = match client
        .post(&endpoint)
        .bearer_auth(&router.api_key)
        .json(&body)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            warn!(model = %router.model, %error, "LLM intent request failed");
            return None;
        }
    };

    // 读取原始正文后再解析，确保成功与失败响应都能出现在日志中。
    let status = response.status();
    let response_body = match response.text().await {
        Ok(body) => body,
        Err(error) => {
            warn!(model = %router.model, %error, "Failed to read LLM intent response body");
            return None;
        }
    };
    info!(
        model = %router.model,
        endpoint = %endpoint,
        status = %status,
        response_body = %response_body,
        "Received LLM intent response"
    );
    if !status.is_success() {
        warn!(model = %router.model, %status, "LLM intent response returned an error status");
        return None;
    }

    // 将 2xx 正文反序列化为 OpenAI 兼容的 ChatResponse 结构。
    let response = match serde_json::from_str::<ChatResponse>(&response_body) {
        Ok(response) => response,
        Err(error) => {
            warn!(model = %router.model, %error, "Failed to parse LLM intent response");
            return None;
        }
    };

    let Some(choice) = response.choices.first() else {
        warn!(model = %router.model, "LLM intent response contains no choices");
        return None;
    };
    let intent = parse_response(&choice.message.content, catalog, allowed);
    if intent.is_none() {
        warn!(model = %router.model, "LLM returned an invalid or low-confidence intent");
    }
    intent
}

fn parse_response(content: &str, catalog: &IntentCatalog, allowed: &[&str]) -> Option<Intent> {
    // 兼容部分模型用 Markdown 代码块包裹 JSON 的情况。
    let content = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    // JSON 结构、置信度和 YAML 白名单均需在本地再次校验，模型结果不被直接信任。
    let output: ModelIntent = serde_json::from_str(content).ok()?;
    if !(MIN_CONFIDENCE..=1.0).contains(&output.confidence) {
        return None;
    }
    // 模型只能返回 YAML 中已声明的意图名称。
    catalog.find(&output.intent)?;
    if !allowed.is_empty() && !allowed.contains(&output.intent.as_str()) {
        return None;
    }
    let cli = match output.cli.as_deref() {
        None | Some("null") => None,
        Some("codex") => Some(AgentKind::Codex),
        Some("cursor") => Some(AgentKind::Cursor),
        Some(_) => return None,
    };
    if output.intent == "choose_cli" && cli.is_none() {
        return None;
    }
    Some(Intent::new(output.intent).with_cli(cli))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> IntentCatalog {
        IntentCatalog::load("config/intents.yaml").unwrap()
    }
    #[test]
    fn accepts_valid_choose_cli_json() {
        assert_eq!(
            parse_response(
                r#"{"intent":"choose_cli","cli":"codex","confidence":0.9}"#,
                &catalog(),
                &[],
            )
            .unwrap()
            .cli,
            Some(AgentKind::Codex)
        );
    }
    #[test]
    fn rejects_low_confidence_json() {
        assert!(parse_response(
            r#"{"intent":"close_terminal","confidence":0.2}"#,
            &catalog(),
            &[],
        )
        .is_none());
    }
}

//! OpenAI Chat Completions 兼容客户端。

use crate::infrastructure::config::RouterConfig;
use std::time::Duration;
use tracing::{info, warn};

/// 发起一次模型请求并返回首个候选消息正文。
pub async fn complete(
    router: &RouterConfig,
    temperature: f32,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    if !router.enabled || router.api_key.is_empty() {
        return Err("模型路由未配置".to_owned());
    }
    let endpoint = format!("{}/chat/completions", router.base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(router.timeout_secs))
        .build()
        .map_err(|error| format!("无法创建模型客户端：{error}"))?;
    let body = serde_json::json!({
        "model": router.model,
        "temperature": temperature,
        "messages": messages,
    });
    info!(model = %router.model, endpoint = %endpoint, "Sending LLM completion request");
    let response = client
        .post(&endpoint)
        .bearer_auth(&router.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            warn!(model = %router.model, %error, "LLM completion request failed");
            "模型请求失败".to_owned()
        })?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|_| "读取模型响应失败".to_owned())?;
    info!(model = %router.model, %status, "Received LLM completion response");
    if !status.is_success() {
        return Err("模型服务不可用".to_owned());
    }
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| "模型响应格式错误".to_owned())?;
    value["choices"]
        .get(0)
        .and_then(|choice| choice["message"]["content"].as_str())
        .map(str::trim)
        .filter(|content| !content.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "模型没有返回内容".to_owned())
}

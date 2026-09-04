//! Telegram HTTP 客户端工厂：集中处理代理与网络连接配置。

use teloxide::Bot;
use tracing::info;

use crate::infrastructure::{config::TelegramConfig, http::build_http_client};

/// 根据 Telegram 配置创建带代理支持的 Bot 客户端。
pub fn build(config: &TelegramConfig) -> Result<Bot, reqwest::Error> {
    let proxy_url = config
        .proxy_url
        .as_deref()
        .filter(|url| !url.trim().is_empty());
    let client = build_http_client(proxy_url)?;
    if let Some(proxy_url) = proxy_url {
        info!(proxy_url, "Using configured Telegram proxy");
    }
    Ok(Bot::with_client(&config.token, client))
}

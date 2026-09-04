use anyhow::{Context, Result};
use serde::Deserialize;
use std::{path::Path, sync::Arc};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub telegram: TelegramConfig,
    pub log: LogConfig,
    #[serde(default)]
    /// 应用全局共享的模型路由配置。
    pub router: Arc<RouterConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelegramConfig {
    pub token: String,
    pub allowed_user_ids: Vec<i64>,
    /// Optional HTTP proxy, for example `http://127.0.0.1:7890`.
    #[serde(default)]
    pub proxy_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LogConfig {
    pub dir: String,
    pub level: String,
}

/// OpenAI-compatible chat model used only for manager intent routing.
#[derive(Debug, Deserialize, Clone)]
pub struct RouterConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: default_base_url(),
            api_key: String::new(),
            model: default_model(),
            timeout_secs: default_timeout_secs(),
        }
    }
}

fn default_base_url() -> String {
    "https://api.openai.com/v1".into()
}

fn default_model() -> String {
    "gpt-4o-mini".into()
}

fn default_timeout_secs() -> u64 {
    60
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        let mut config: Config =
            serde_yaml::from_str(&content).context("Failed to parse config.yaml")?;

        // Override token from environment variable if set
        if let Ok(token) = std::env::var("TELEGRAM_BOT_TOKEN") {
            if !token.is_empty() {
                config.telegram.token = token;
            }
        }
        if let Ok(api_key) = std::env::var("ROUTER_API_KEY") {
            if !api_key.is_empty() {
                Arc::make_mut(&mut config.router).api_key = api_key;
            }
        }

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.telegram.token.is_empty() {
            anyhow::bail!(
                "Telegram bot token is not set. \
                Set it in config.yaml or via TELEGRAM_BOT_TOKEN env var."
            );
        }
        if self.telegram.allowed_user_ids.is_empty() {
            anyhow::bail!("allowed_user_ids must not be empty.");
        }
        if self.router.enabled && self.router.api_key.is_empty() {
            anyhow::bail!(
                "router.enabled is true but api_key is empty. \
                Set router.api_key in config.yaml or via ROUTER_API_KEY env var."
            );
        }
        if self.router.enabled && self.router.base_url.trim().is_empty() {
            anyhow::bail!("router.base_url must not be empty when router.enabled is true.");
        }
        if self.router.enabled && self.router.model.trim().is_empty() {
            anyhow::bail!("router.model must not be empty when router.enabled is true.");
        }
        Ok(())
    }
}

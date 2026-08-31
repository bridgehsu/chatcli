use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub telegram: TelegramConfig,
    pub log: LogConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelegramConfig {
    pub token: String,
    pub allowed_user_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LogConfig {
    pub dir: String,
    pub level: String,
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
        Ok(())
    }
}

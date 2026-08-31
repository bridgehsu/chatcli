mod agent;
mod cli;
mod config;
mod instance;
mod telegram;
mod terminal;

use std::sync::Arc;

use anyhow::Result;
use tracing::info;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use config::Config;
use instance::InstanceGuard;
use telegram::start_bot;

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = InstanceGuard::acquire()?;

    // Load config
    let config = Arc::new(Config::load("config.yaml")?);

    // Set up logging
    init_logging(&config)?;

    info!("ChatCLI starting from the current user's home directory");

    // Start Telegram bot (blocks until shutdown)
    start_bot(Arc::clone(&config)).await;

    Ok(())
}

fn init_logging(config: &Config) -> Result<()> {
    std::fs::create_dir_all(&config.log.dir)?;

    let level_filter =
        EnvFilter::try_new(&config.log.level).unwrap_or_else(|_| EnvFilter::new("info"));

    // Rolling daily log files
    let file_appender = rolling::daily(&config.log.dir, "chatcli.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Keep _guard alive for the process lifetime by leaking it (intentional)
    std::mem::forget(_guard);

    tracing_subscriber::registry()
        .with(level_filter)
        .with(
            // Console output
            fmt::layer().with_target(false).compact(),
        )
        .with(
            // File output (JSON for structured parsing)
            fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_target(true),
        )
        .init();

    Ok(())
}

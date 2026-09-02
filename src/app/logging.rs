use anyhow::Result;
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::infrastructure::config::LogConfig;

/// Initializes process-wide logging and returns a guard that must live until exit.
pub fn init(config: &LogConfig) -> Result<WorkerGuard> {
    std::fs::create_dir_all(&config.dir)?;

    let level_filter = EnvFilter::try_new(&config.level).unwrap_or_else(|_| EnvFilter::new("info"));
    let file_appender = rolling::daily(&config.dir, "chatcli.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(level_filter)
        .with(fmt::layer().with_target(false).compact())
        .with(
            fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_target(true),
        )
        .init();

    Ok(guard)
}

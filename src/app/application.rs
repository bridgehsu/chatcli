use std::sync::Arc;

use anyhow::Result;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;

use crate::{
    app::logging,
    channels::telegram::start,
    infrastructure::{config::Config, instance::InstanceGuard},
};

/// Application bootstrapper. Equivalent to Spring Boot's application runner:
/// it assembles process-wide infrastructure before starting the service layer.
pub struct ChatCliApplication {
    config: Arc<Config>,
    _instance_guard: InstanceGuard,
    _logging_guard: WorkerGuard,
}

impl ChatCliApplication {
    pub fn new() -> Result<Self> {
        let config = Arc::new(Config::load("config.yaml")?);
        let logging_guard = logging::init(&config.log)?;
        let instance_guard = InstanceGuard::acquire()?;

        info!("ChatCLI application started");
        Ok(Self {
            config,
            _instance_guard: instance_guard,
            _logging_guard: logging_guard,
        })
    }

    pub async fn run(self) -> Result<()> {
        start(self.config).await;
        Ok(())
    }
}

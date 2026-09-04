use std::sync::Arc;

use anyhow::Result;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;

use crate::{
    agent::intent::IntentCatalog,
    app::{logging, AppState},
    infrastructure::{config::Config, instance::InstanceGuard},
    interfaces::registry::ChannelRegistry,
};

/// Application bootstrapper. Equivalent to Spring Boot's application runner:
/// it assembles process-wide infrastructure before starting the service layer.
pub struct ChatCliApplication {
    config: Arc<Config>,
    intent_catalog: Arc<IntentCatalog>,
    _instance_guard: InstanceGuard,
    _logging_guard: WorkerGuard,
}

impl ChatCliApplication {
    pub fn new() -> Result<Self> {
        let config = Arc::new(Config::load("config.yaml")?);
        let intent_catalog = Arc::new(IntentCatalog::load("config/intents.yaml")?);
        let logging_guard = logging::init(&config.log)?;
        let instance_guard = InstanceGuard::acquire()?;

        info!("ChatCLI application started");
        Ok(Self {
            config,
            intent_catalog,
            _instance_guard: instance_guard,
            _logging_guard: logging_guard,
        })
    }

    pub async fn run(self) -> Result<()> {
        let state = Arc::new(AppState::new(self.config, self.intent_catalog).await?);
        ChannelRegistry::from_config(&state).start_all(state).await
    }
}

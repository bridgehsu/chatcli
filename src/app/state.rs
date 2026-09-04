//! 进程级共享依赖：一个应用实例只创建一个 Agent。

use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};

use crate::{agent::intent::IntentCatalog, app::Agent, infrastructure::config::Config};

/// 全局应用状态。各通道只使用它，不负责创建业务服务。
pub struct AppState {
    pub config: Arc<Config>,
    pub agent: Arc<Agent>,
}

impl AppState {
    pub async fn new(config: Arc<Config>, catalog: Arc<IntentCatalog>) -> Result<Self> {
        let home: PathBuf = dirs::home_dir().context("无法确定当前用户的 Home 目录")?;
        let agent = Arc::new(Agent::new(home, Arc::clone(&config.router), catalog).await?);
        Ok(Self { config, agent })
    }
}

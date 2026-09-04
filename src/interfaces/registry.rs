//! 已启用外部渠道的注册和生命周期管理。

use crate::{
    app::AppState,
    interfaces::channel::{Channel, ChannelRegistration},
};
use anyhow::Result;
use std::sync::Arc;
use tracing::info;

pub struct ChannelRegistry {
    channels: Vec<Box<dyn Channel>>,
}

impl ChannelRegistry {
    pub fn from_config(state: &AppState) -> Self {
        // 渠道由 Adapter 自注册；新增渠道无需修改 app 层。
        Self {
            channels: inventory::iter::<ChannelRegistration>
                .into_iter()
                .filter(|registration| (registration.enabled)(&state.config))
                .map(|registration| {
                    info!(channel = registration.name, "Registered configured channel");
                    (registration.create)()
                })
                .collect(),
        }
    }

    pub async fn start_all(self, state: Arc<AppState>) -> Result<()> {
        let mut tasks = Vec::with_capacity(self.channels.len());
        for channel in self.channels {
            info!(channel = channel.name(), "Starting configured channel");
            tasks.push(tokio::spawn(channel.start(Arc::clone(&state))));
        }
        for task in tasks {
            task.await??;
        }
        Ok(())
    }
}

mod bot;
mod client;
mod output;
pub mod presenter;
pub mod terminal_view;
use crate::{
    app::AppState,
    infrastructure::config::Config,
    interfaces::channel::{Channel, ChannelRegistration},
};
use anyhow::Result;
pub use output::TelegramOutput;
use std::{future::Future, pin::Pin, sync::Arc};

/// Telegram 的 Channel 适配器；未来飞书、钉钉分别实现同一个 Channel 接口。
pub struct TelegramChannel;

impl Channel for TelegramChannel {
    fn name(&self) -> &'static str {
        "telegram"
    }

    fn start(
        self: Box<Self>,
        state: Arc<AppState>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            bot::start(state).await;
            Ok(())
        })
    }
}

fn enabled(config: &Config) -> bool {
    !config.telegram.token.trim().is_empty()
}

fn create() -> Box<dyn Channel> {
    Box::new(TelegramChannel)
}

inventory::submit! {
    ChannelRegistration { name: "telegram", enabled, create }
}

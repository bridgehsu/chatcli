//! Telegram 对 `service::output::ChatOutput` 的实现。

use crate::service::output::ChatOutput;
use anyhow::Result;
use std::{future::Future, pin::Pin};
use teloxide::{prelude::*, types::ChatId};

#[derive(Clone)]
pub struct TelegramOutput {
    bot: Bot,
}

impl TelegramOutput {
    pub fn new(bot: Bot) -> Self {
        Self { bot }
    }
}

impl ChatOutput for TelegramOutput {
    fn send<'a>(
        &'a self,
        chat_id: i64,
        text: String,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            self.bot.send_message(ChatId(chat_id), text).await?;
            Ok(())
        })
    }
}

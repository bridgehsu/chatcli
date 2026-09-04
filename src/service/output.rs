//! Application Service 对外发送消息的渠道无关端口。

use anyhow::Result;
use std::{future::Future, pin::Pin};

/// 业务层只描述“向某个聊天发送文本”，不感知 Telegram、飞书或钉钉 SDK。
pub trait ChatOutput: Send + Sync {
    fn send<'a>(
        &'a self,
        chat_id: i64,
        text: String,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;
}

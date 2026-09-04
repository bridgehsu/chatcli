//! 渠道端口：Telegram、飞书、钉钉等外部消息平台都实现此接口。

use crate::{app::AppState, infrastructure::config::Config};
use anyhow::Result;
use std::{future::Future, pin::Pin, sync::Arc};

pub trait Channel: Send + Sync {
    fn name(&self) -> &'static str;

    /// 启动渠道的接收循环。渠道只负责平台协议适配，不创建 Agent 或业务服务。
    fn start(
        self: Box<Self>,
        state: Arc<AppState>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;
}

pub struct ChannelRegistration {
    pub name: &'static str,
    pub enabled: fn(&Config) -> bool,
    pub create: fn() -> Box<dyn Channel>,
}

inventory::collect!(ChannelRegistration);

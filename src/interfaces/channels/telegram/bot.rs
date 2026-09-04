//! Telegram transport adapter. Business behavior lives in `crate::agent::Agent`.

use std::{sync::Arc, time::Duration};

use teloxide::{
    dispatching::{Dispatcher, UpdateFilterExt},
    error_handlers::LoggingErrorHandler,
    prelude::*,
    types::Update,
    update_listeners,
};
use tracing::{debug, error, info, warn};

use crate::{
    app::{Agent, AppState},
    interfaces::channels::telegram::{client, TelegramOutput},
};

/// 启动 Telegram 通道。通道只转发消息，不创建或管理全局 Agent。
pub async fn start(state: Arc<AppState>) {
    let bot = match client::build(&state.config.telegram) {
        Ok(bot) => bot,
        Err(error) => {
            error!(?error, "Failed to create Telegram HTTP client");
            return;
        }
    };
    info!("ChatCLI Telegram adapter starting");
    run_polling(
        bot,
        Arc::clone(&state.agent),
        state.config.telegram.allowed_user_ids.clone(),
    )
    .await;
}

/// 创建消息处理器，并在网络失败后重新建立 Telegram long-polling 连接。
async fn run_polling(bot: Bot, agent: Arc<Agent>, allowed: Vec<i64>) {
    let handler = Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
        let agent = Arc::clone(&agent);
        let allowed = allowed.clone();
        async move { handle_message(agent, allowed, bot, msg).await }
    });

    // `teloxide::repl` 会在初始 getMe 请求失败时 panic；这里显式重试网络错误。
    loop {
        let listener = update_listeners::polling_default(bot.clone()).await;
        let mut dispatcher = Dispatcher::builder(bot.clone(), handler.clone())
            .default_handler(|_| Box::pin(async {}))
            .enable_ctrlc_handler()
            .build();
        match dispatcher
            .try_dispatch_with_listener(
                listener,
                LoggingErrorHandler::with_custom_text("An error from the update listener"),
            )
            .await
        {
            Ok(()) => {
                info!("Telegram polling stopped");
                return;
            }
            Err(error) => {
                error!(?error, "Telegram connection failed; retrying in 10 seconds");
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

/// 校验来源、提取有效文本，再交给全局 Agent 执行业务流程。
async fn handle_message(
    agent: Arc<Agent>,
    allowed: Vec<i64>,
    bot: Bot,
    msg: Message,
) -> Result<(), teloxide::RequestError> {
    let user = msg
        .from
        .as_ref()
        .map(|user| user.id.0 as i64)
        .unwrap_or_default();
    let chat = msg.chat.id;
    if !allowed.contains(&user) {
        warn!(
            user,
            chat_id = chat.0,
            "Rejected unauthorized Telegram message"
        );
        return Ok(());
    }
    let Some(text) = msg.text().map(str::trim).filter(|text| !text.is_empty()) else {
        debug!(
            user,
            chat_id = chat.0,
            "Ignored non-text or empty Telegram update"
        );
        return Ok(());
    };
    info!(
        user,
        chat_id = chat.0,
        text_len = text.chars().count(),
        "Received Telegram message"
    );
    let output = Arc::new(TelegramOutput::new(bot.clone()));
    agent.run(bot, output, chat, text).await;
    Ok(())
}

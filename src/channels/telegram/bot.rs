//! Telegram transport adapter. Business behavior lives in `crate::agent::Agent`.

use std::sync::Arc;

use teloxide::prelude::*;
use tracing::{info, warn};

use crate::{agent::Agent, infrastructure::config::Config};

pub async fn start(config: Arc<Config>) {
    let home = dirs::home_dir().expect("无法确定当前用户的 Home 目录");
    let agent = Arc::new(
        Agent::new(home, Arc::clone(&config.router))
            .await
            .expect("Failed to initialize ChatCLI agent"),
    );
    let allowed = config.telegram.allowed_user_ids.clone();
    let bot = Bot::new(&config.telegram.token);
    info!("ChatCLI Telegram adapter starting");

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let agent = Arc::clone(&agent);
        let allowed = allowed.clone();
        async move {
            let user = msg
                .from
                .as_ref()
                .map(|user| user.id.0 as i64)
                .unwrap_or_default();
            if !allowed.contains(&user) {
                warn!(user, "Rejected unauthorized Telegram message");
                return Ok(());
            }
            let Some(text) = msg.text().map(str::trim).filter(|text| !text.is_empty()) else {
                return Ok(());
            };
            agent.run(bot, msg.chat.id, text).await;
            Ok(())
        }
    })
    .await;
}

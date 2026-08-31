//! Telegram rendering for one tmux terminal.
//!
//! This module owns polling and message updates only. It does not decide which
//! terminal to create or how long that terminal should live.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use teloxide::{
    prelude::*,
    types::{MessageId, ParseMode},
};
use tokio::{sync::oneshot, task::JoinHandle};

use crate::cli::CliRunner;

use super::presenter;

pub fn spawn(
    bot: Bot,
    chat_id: ChatId,
    runner: Arc<CliRunner>,
) -> (oneshot::Sender<()>, JoinHandle<()>) {
    let (stop, mut stop_rx) = oneshot::channel();
    let watcher = tokio::spawn(async move {
        let mut previous = String::new();
        let mut message: Option<MessageId> = None;
        let mut last_update = Instant::now() - Duration::from_secs(2);

        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                _ = tokio::time::sleep(Duration::from_millis(700)) => {}
            };
            let Ok(screen) = runner.capture_pane_public().await else {
                continue;
            };
            let view = presenter::tail_chars(&presenter::strip_ansi(&screen), 3000);
            if view.is_empty()
                || view == previous
                || last_update.elapsed() < Duration::from_millis(1100)
            {
                continue;
            }
            previous = view.clone();
            last_update = Instant::now();
            let body = format!(
                "<b>[终端]</b>\n<pre>{}</pre>",
                presenter::html_escape(&view)
            );
            if let Some(id) = message {
                if bot
                    .edit_message_text(chat_id, id, &body)
                    .parse_mode(ParseMode::Html)
                    .await
                    .is_ok()
                {
                    continue;
                }
            }
            message = bot
                .send_message(chat_id, body)
                .parse_mode(ParseMode::Html)
                .await
                .ok()
                .map(|message| message.id);
        }
    });
    (stop, watcher)
}

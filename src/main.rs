use anyhow::Result;

mod agent;
mod app;
mod channels;
mod infrastructure;
mod manager;
mod utils;

use app::ChatCliApplication;

#[tokio::main]
async fn main() -> Result<()> {
    ChatCliApplication::new()?.run().await
}

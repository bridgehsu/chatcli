use anyhow::Result;

mod agent;
mod app;
mod channels;
mod infrastructure;
mod session;
mod tools;

use app::ChatCliApplication;

#[tokio::main]
async fn main() -> Result<()> {
    ChatCliApplication::new()?.run().await
}

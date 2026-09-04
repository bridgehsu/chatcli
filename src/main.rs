use anyhow::Result;

mod agent;
mod app;
mod domain;
mod infrastructure;
mod interfaces;
mod service;
mod shared;
mod tools;

use app::ChatCliApplication;

#[tokio::main]
async fn main() -> Result<()> {
    ChatCliApplication::new()?.run().await
}

use std::sync::Arc;

use tracing_subscriber::EnvFilter;
use ultron_discord::DiscordBot;

/// panics if a subscriber was already registered.
/// configure log levels with the RUST_LOG environment variable.
fn setup_tracing() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .with_current_span(true)
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();
    tracing::info!("starting ultron");
    let discord_bot = DiscordBot::new(Arc::new(ultron_core::EventProcessor))?;

    discord_bot.run().await?;

    tracing::info!("bye");
    Ok(())
}

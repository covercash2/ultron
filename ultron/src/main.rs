use std::sync::Arc;

use tracing_subscriber::EnvFilter;
use ultron_discord::DiscordBotConfig;

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
    let discord_bot = DiscordBotConfig::new(Arc::new(ultron_core::EventProcessor))?;

    let bot = discord_bot.run().await?;

    bot.debug("coming online").await?;
    bot.psa("coming online").await?;

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("received ctrl-c, shutting down");
        }
    }

    tracing::info!("bye");
    Ok(())
}

use std::sync::Arc;

use tracing_subscriber::EnvFilter;
use ultron_core::http_server::{self, AppState};
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
    let event_processor = Arc::new(ultron_core::EventProcessor);

    let discord_config = DiscordBotConfig::new(event_processor.clone())?;

    let bot = Arc::new(discord_config.run().await?);

    bot.debug("coming online").await?;

    let discord_thread_bot = bot.clone();
    let server_thread_bot = bot.clone();
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("received ctrl-c, shutting down");
            discord_thread_bot.shutdown().await?;
        }
        _ = http_server::serve(8080, AppState {
            event_processor,
            chat_bot: server_thread_bot.clone(),
        }) => {
            tracing::warn!("http server shut down spontaneously");
        }
    }

    tracing::info!("bye");
    Ok(())
}

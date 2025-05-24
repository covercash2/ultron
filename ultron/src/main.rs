use std::sync::Arc;

use clap::Parser;
use tracing_subscriber::EnvFilter;
use ultron_core::http_server::{self, AppState};
use ultron_discord::DiscordBotConfig;

/// environment variables
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Env {
    pub discord_app_id: String,
    pub discord_public_key: String,
    pub discord_token: String,
}

/// CLI args
#[derive(Debug, Clone, Parser)]
pub struct Cli {
    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    #[arg(short, long, default_value = "info")]
    pub rust_log: String,
}

/// panics if a subscriber was already registered.
/// configure log levels with the RUST_LOG environment variable.
fn setup_tracing(rust_log: &str) {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_new(rust_log).expect("unable to build EnvFilter"))
        .with_line_number(true)
        .with_current_span(true)
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    setup_tracing(&args.rust_log);
    tracing::info!("starting ultron");
    dbg!("starting ultron");

    let env = envy::from_env::<Env>()?;
    tracing::info!("log level: {}", args.rust_log);

    tracing::info!("CLI args: {args:?}");

    let event_processor = Arc::new(ultron_core::EventProcessor);

    let discord_config = DiscordBotConfig::builder()
        .application_id(env.discord_app_id)
        .token(env.discord_token)
        .public_key(env.discord_public_key)
        .event_processor(event_processor.clone())
        .build();

    let bot = Arc::new(discord_config.run().await?);

    bot.debug("coming online").await?;

    let discord_thread_bot = bot.clone();
    let server_thread_bot = bot.clone();
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("received ctrl-c, shutting down");
            discord_thread_bot.shutdown().await?;
        }
        result = http_server::serve(args.port, AppState {
            event_processor,
            chat_bot: server_thread_bot.clone(),
        }) => {
            tracing::warn!("http server shut down spontaneously: {:?}", result);
        }
    }

    tracing::info!("bye");
    Ok(())
}

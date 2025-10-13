use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tracing_subscriber::EnvFilter;
use ultron_core::{
    chatbot::ChatBot,
    command::CommandConsumer,
    dice::DiceRoller,
    event_processor::EventProcessor,
    http_server::{self, AppState},
    io::read_file_to_string,
    nlp::{ChatAgentConfig, LmChatAgent},
};
use ultron_discord::DiscordBotConfig;

#[derive(Clone, serde::Deserialize)]
pub struct Secrets {
    pub discord_app_id: String,
    pub discord_public_key: String,
    pub discord_token: String,
}

/// CLI args
#[derive(Debug, Clone, Parser)]
pub struct Cli {
    /// port to listen on for HTTP requests
    #[arg(short, long)]
    pub port: u16,

    /// log level in the form of the [`env_logger`] crate
    ///
    /// [`env_logger`]: https://docs.rs/env_logger/latest/env_logger/#enabling-logging
    #[arg(short, long, default_value = "info")]
    pub rust_log: String,

    /// host to connect to the language model
    #[arg(short, long, default_value = "https://hoss.faun-truck.ts.net/llm/")]
    pub lm_endpoint: String,

    /// path to the Ultron MCP server
    #[arg(short, long)]
    pub mcp_port: u16,

    /// path to the secrets file
    #[arg(short, long)]
    pub secrets: PathBuf,
}

impl From<&Cli> for ChatAgentConfig {
    fn from(value: &Cli) -> Self {
        Self {
            llm_uri: value.lm_endpoint.clone(),
            llm_model: "llama3.2:latest".into(),
            mcp_uri: format!("http://localhost:{}", value.mcp_port),
        }
    }
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

    let contents = read_file_to_string(&args.secrets)
        .await
        .inspect_err(|error| {
            tracing::error!(
                %error,
                "unable to read secrets file from CLI args",
            );
        })?;

    let secrets: Secrets = toml::from_str(&contents)?;

    tracing::info!("log level: {}", args.rust_log);

    tracing::info!("CLI args: {args:?}");

    let event_processor =
        EventProcessor::new().with_consumer(CommandConsumer::new(DiceRoller::default()));

    let event_processor: Arc<EventProcessor> = if let Ok(chat_agent) = LmChatAgent::load((&args).into()).await
        .inspect_err(|error| {
            tracing::error!(%error, "!!! unable to create chat agent !!!");
            tracing::error!(%error, "the server will continue to run, but LLM capabilities will be unavailable");
        }) {
        event_processor.with_consumer(chat_agent).into()
    } else {
        event_processor.into()
    };

    let discord_config = DiscordBotConfig::builder()
        .application_id(secrets.discord_app_id)
        .token(secrets.discord_token)
        .public_key(secrets.discord_public_key)
        .event_processor(event_processor.clone())
        .build();

    let bot = Arc::new(discord_config.run().await?);

    let hostname = read_file_to_string("/etc/hostname")
        .await?
        .trim()
        .to_string();

    let startup_message = format!("coming online, listening from {hostname}:{}", args.port);

    bot.debug(&startup_message).await?;

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

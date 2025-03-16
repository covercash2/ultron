use bon::Builder;
use serenity::{
    Client,
    all::{Context, EventHandler, GatewayIntents, Message},
};
use std::sync::Arc;
use thiserror::Error;
use ultron_core::{ChatInput, Event, EventProcessor, Response};

pub type DiscordBotResult<T> = Result<T, DiscordBotError>;

#[derive(Debug, Clone, Error)]
pub enum DiscordBotError {
    #[error("missing application id. set APP_ID environment variable")]
    MissingAppId,
    #[error("missing token. set DISCORD_TOKEN environment variable")]
    MissingToken,
    #[error("missing public key. set PUBLIC_KEY environment variable")]
    MissingPublicKey,
    #[error("could not get owner from Discord")]
    EmptyOwner,
}

#[derive(Builder, Debug, Clone)]
pub struct DiscordBot {
    #[builder(into)]
    application_id: String,
    #[builder(into)]
    token: String,
    #[builder(into)]
    public_key: String,
    #[builder(default)]
    intents: Intents,
    event_processor: Arc<EventProcessor>,
}

impl DiscordBot {
    pub fn new(event_processor: Arc<EventProcessor>) -> DiscordBotResult<Self> {
        let bot = DiscordBot::builder()
            .token(std::env::var("DISCORD_TOKEN").map_err(|_| DiscordBotError::MissingToken)?)
            .application_id(std::env::var("APP_ID").map_err(|_| DiscordBotError::MissingAppId)?)
            .public_key(std::env::var("PUBLIC_KEY").map_err(|_| DiscordBotError::MissingPublicKey)?)
            .event_processor(event_processor)
            .build();

        tracing::info!("using default intents: {:?}", bot.intents);

        Ok(bot)
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let http = serenity::http::Http::new(&self.token);

        let app_info = http
            .get_current_application_info()
            .await?
            .owner
            .ok_or(DiscordBotError::EmptyOwner)?;

        tracing::info!("got app info: {:?}", app_info);

        let mut client = Client::builder(&self.token, self.intents.0)
            .application_id(self.application_id.parse()?)
            .event_handler(Handler {
                event_processor: self.event_processor,
            })
            .await?;

        client.start().await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Intents(GatewayIntents);

impl Default for Intents {
    fn default() -> Self {
        let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
        Self(intents)
    }
}

struct Handler {
    event_processor: Arc<EventProcessor>,
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let event: ChatInput = msg.content.into();
        let event: Event = Event::ChatInput(event);
        let Response::Plain(response) = self.event_processor.process(event).await;

        if let Err(error) = msg.channel_id.say(&ctx.http, response).await {
            tracing::error!("Error sending message: {:?}", error);
        }
    }
}

use bon::Builder;
use serenity::{
    Client,
    all::{ChannelId, Context, EventHandler, GatewayIntents, Message},
    http::Http,
};
use std::sync::Arc;
use thiserror::Error;
use tokio::task::JoinHandle;
use ultron_core::{
    Channel, ChatBot, ChatInput, Event, EventError, EventProcessor, Response,
    command::CommandParseError, dice::HELP_MESSAGE,
};

/// ultron#ultron-test
const DEFAULT_DEBUG_CHANNEL_ID: ChannelId = ChannelId::new(777725275856699402);
const DEFAULT_GENERAL_CHANNEL_ID: ChannelId = ChannelId::new(777658379212161077);
const DEFAULT_DND_CHANNEL_ID: ChannelId = ChannelId::new(874085144284258325);

pub type DiscordBotResult<T> = Result<T, DiscordBotError>;

#[derive(Debug, Error)]
pub enum DiscordBotError {
    #[error("missing application id. set APP_ID environment variable")]
    MissingAppId,
    #[error("missing token. set DISCORD_TOKEN environment variable")]
    MissingToken,
    #[error("missing public key. set PUBLIC_KEY environment variable")]
    MissingPublicKey,
    #[error("could not get owner from Discord")]
    EmptyOwner,
    #[error(transparent)]
    Serenity(#[from] serenity::Error),
    #[error("content is too long")]
    ContentTooLong,
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
}

#[derive(Builder, Debug, Clone)]
pub struct DiscordBotConfig {
    #[builder(into)]
    application_id: String,
    #[builder(into)]
    token: String,
    /// TODO: i'm not sure why i need this field, but it's here
    #[allow(unused)]
    #[builder(into)]
    public_key: String,
    #[builder(default)]
    intents: Intents,
    event_processor: Arc<EventProcessor>,
}

impl DiscordBotConfig {
    pub async fn run(self) -> anyhow::Result<DiscordBot> {
        let http = Http::new(&self.token);

        let app_info = http.get_current_application_info().await?;

        tracing::info!("got app info: {:?}", app_info);

        let client_handle = tokio::spawn(async move {
            let mut client = Client::builder(&self.token, self.intents.0)
                .application_id(self.application_id.parse().unwrap())
                .event_handler(Handler {
                    event_processor: self.event_processor,
                })
                .await?;

            tracing::info!("starting Discord client");

            client.start().await?;

            Ok(())
        })
        .into();

        Ok(DiscordBot {
            http: Arc::new(http),
            client_handle,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DiscordBot {
    http: Arc<Http>,
    client_handle: Arc<JoinHandle<DiscordBotResult<()>>>,
}

pub struct DiscordChannel(ChannelId);

impl From<Channel> for DiscordChannel {
    fn from(channel: Channel) -> Self {
        match channel {
            Channel::Debug => DiscordChannel(DEFAULT_DEBUG_CHANNEL_ID),
            Channel::Psa => DiscordChannel(DEFAULT_GENERAL_CHANNEL_ID),
            Channel::Dnd => DiscordChannel(DEFAULT_DND_CHANNEL_ID),
        }
    }
}

impl ChatBot for DiscordBot {
    type Error = DiscordBotError;

    async fn send_message(&self, channel: Channel, message: &str) -> DiscordBotResult<()> {
        if message.len() >= 2000 {
            return Err(DiscordBotError::ContentTooLong);
        }
        let DiscordChannel(id) = channel.into();

        id.say(&self.http, message).await?;

        Ok(())
    }
}

impl DiscordBot {
    /// send a message to the debug channel
    pub async fn debug(&self, message: &str) -> DiscordBotResult<()> {
        self.send_message(DEFAULT_DEBUG_CHANNEL_ID, message).await
    }

    /// send a message to the general channel
    pub async fn psa(&self, message: &str) -> DiscordBotResult<()> {
        self.send_message(DEFAULT_GENERAL_CHANNEL_ID, message).await
    }

    /// send a message to the dnd channel
    pub async fn dnd(&self, message: &str) -> DiscordBotResult<()> {
        self.send_message(DEFAULT_DND_CHANNEL_ID, message).await
    }

    pub async fn send_message(&self, channel_id: ChannelId, message: &str) -> DiscordBotResult<()> {
        if message.len() >= 2000 {
            return Err(DiscordBotError::ContentTooLong);
        }

        channel_id.say(&self.http, message).await?;

        Ok(())
    }

    pub async fn shutdown(&self) -> DiscordBotResult<()> {
        self.client_handle.abort();
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
        tracing::debug!("handling message event: {:?}", msg);

        let event: ChatInput = msg.content.into();
        let event: Event = Event::ChatInput(event);

        match self.event_processor.process(event).await {
            Ok(Response::PlainChat(response)) => {
                if let Err(error) = msg.channel_id.say(&ctx.http, response).await {
                    tracing::error!(%error, "error sending message");
                }
            }
            Err(error) => {
                tracing::error!(%error, "error processing event");

                let error_message = match error {
                    EventError::CommandParse(command_parse_error) => match command_parse_error {
                        CommandParseError::MissingCommand(error_msg)
                        | CommandParseError::UndefinedCommand(error_msg) => {
                            Some(format!("ya blew it: {}\n\n{}", error_msg, HELP_MESSAGE))
                        }
                        _ => None,
                    },
                    EventError::DiceRollParse(dice_roll_error) => Some(format!(
                        "ya blew it: {}\n\n{}",
                        dice_roll_error, HELP_MESSAGE
                    )),
                };

                if let Some(error_message) = error_message {
                    if let Err(error) = msg.channel_id.say(&ctx.http, error_message).await {
                        tracing::error!(%error, "error sending message");
                    }
                }
            }
        }
    }
}

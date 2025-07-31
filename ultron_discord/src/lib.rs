use bon::Builder;
use serenity::{
    Client,
    all::{ChannelId, Context, EventHandler, GatewayIntents, Message, UserId},
    http::Http,
};
use std::sync::Arc;
use thiserror::Error;
use tokio::task::JoinHandle;
use ultron_core::{
    Channel, ChatBot, ChatInput, Event, EventError, EventProcessor, EventType, Response, User,
    command::CommandParseError, dice::HELP_MESSAGE,
};

/// ultron#ultron-test
const DEFAULT_DEBUG_CHANNEL_ID: ChannelId = ChannelId::new(777725275856699402);
const DEFAULT_GENERAL_CHANNEL_ID: ChannelId = ChannelId::new(777658379212161077);
const DEFAULT_DND_CHANNEL_ID: ChannelId = ChannelId::new(874085144284258325);

const ULTRON_USER_ID: UserId = UserId::new(777627943144652801);

const DISCORD_MAX_MESSAGE_LENGTH: usize = 2000;

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

        let user = if msg.author.id == ULTRON_USER_ID {
            tracing::debug!("ignoring message from self");
            User::Ultron
        } else {
            User::from(msg.author.name.clone())
        };

        tracing::debug!(user = ?user, "message from user");

        let event_type: EventType = if msg.mentions_user_id(ULTRON_USER_ID) {
            EventType::NaturalLanguage
        } else {
            EventType::Command
        };

        let chat_input: ChatInput = ChatInput {
            user,
            content: msg.content.clone(),
        };

        let event: Event = match Event::new(chat_input, event_type) {
            Ok(event) => event,
            Err(error) => {
                tracing::warn!(%error, "error converting chat input to event");
                return;
            }
        };

        match self.event_processor.process(event.clone()).await {
            Ok(Response::PlainChat(response)) => {
                if response.len() >= DISCORD_MAX_MESSAGE_LENGTH {
                    tracing::debug!(
                        "response is too long ({} characters), truncating to {}",
                        response.len(),
                        DISCORD_MAX_MESSAGE_LENGTH
                    );
                }

                let response_chunks = split_message(&response, DISCORD_MAX_MESSAGE_LENGTH);

                for chunk in response_chunks {
                    if let Err(error) = msg.channel_id.say(&ctx.http, chunk).await {
                        tracing::error!(%error, "error sending message");
                    }
                }
            }
            Err(error) => {
                tracing::error!(
                    ?event,
                    %error,
                    "error processing event",
                );

                let error_message = match error {
                    EventError::CommandParse(command_parse_error) => match command_parse_error {
                        CommandParseError::MissingCommand(error_msg) => {
                            Some(format!("ya blew it: {}\n\n{}", error_msg, HELP_MESSAGE))
                        }
                        CommandParseError::UndefinedCommand { command, args } => Some(format!(
                            "ya blew it: undefined command '{}' with args {:?}\n\n{}",
                            command, args, HELP_MESSAGE
                        )),
                        _ => None,
                    },
                    EventError::LanguageModel(language_model_error) => Some(format!(
                        "brain hurty: {}\n\n{}",
                        language_model_error, HELP_MESSAGE
                    )),
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

/// split a message into chunks of at most `max_length` characters
/// while preserving whole words.
fn split_message(message: &str, max_length: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    for word in message.split_whitespace() {
        if current_chunk.len() + word.len() + 1 > max_length {
            if !current_chunk.is_empty() {
                chunks.push(current_chunk.trim().to_string());
            }
            current_chunk = String::new();
        }
        if !current_chunk.is_empty() {
            current_chunk.push(' ');
        }
        current_chunk.push_str(word);
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_message() {
        let message = "This is a test message that should be split into multiple chunks.";
        let max_length = 20;
        let chunks = split_message(message, max_length);
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0], "This is a test");
        assert_eq!(chunks[1], "message that should");
        assert_eq!(chunks[2], "be split into");
        assert_eq!(chunks[3], "multiple chunks.");
    }

    #[test]
    fn test_split_markdown_syntax() {
        let message = "This is a **test** message with `code` and [link](https://example.com).";
        let max_length = 30;
        let chunks = split_message(message, max_length);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], "This is a **test** message");
        assert_eq!(chunks[1], "with `code` and");
        assert_eq!(chunks[2], "[link](https://example.com).");
    }
}

use bon::Builder;
use extend::ext;
use serenity::{
    Client,
    all::{ChannelId, Context, EventHandler, GatewayIntents, Message, Typing, UserId},
    http::Http,
};
use std::sync::Arc;
use thiserror::Error;
use tokio::task::JoinHandle;
use ultron_core::{
    chatbot::{ChatBot, ChatInput}, command::CommandParseError, dice::HELP_MESSAGE, event_processor::{Event, EventError, EventProcessor, EventType}, Channel, Response, User
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

impl From<DiscordBotError> for ultron_core::error::Error {
    fn from(value: DiscordBotError) -> Self {
        ultron_core::error::Error::ChatBot(Box::new(value))
    }
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
        let DiscordChannel(id) = channel.into();

        id.say(&self.http, message).await?;

        Ok(())
    }
}

impl DiscordBot {
    pub async fn shutdown(&self) -> DiscordBotResult<()> {
        self.client_handle.abort();
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Intents(GatewayIntents);

impl Default for Intents {
    fn default() -> Self {
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_MESSAGE_TYPING;
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
            User::Ultron
        } else {
            User::from(msg.author.name.clone())
        };

        tracing::debug!(user = ?user, "message from user");

        let event_type: EventType = if msg.mentions_ultron() {
            tracing::debug!(user = ?user, "message mentions bot, treating as natural language");
            EventType::LanguageModel
        } else {
            EventType::Plain
        };

        let _typing: Option<Typing> = if event_type == EventType::LanguageModel {
            Some(ctx.http.start_typing(msg.channel_id))
        } else {
            None
        };

        let chat_input = ChatInput {
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

        let results = Box::pin(self.event_processor.process(event.clone())).await;

        let results = match results {
            Ok(results) => results,
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
                    EventError::Agent(agent_error) => {
                        Some(format!("brain hurty: {agent_error}\n{HELP_MESSAGE}",))
                    }
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

                return;
            }
        };

        tracing::debug!(?results, "processing event result");

        for result in results {
            match result {
                Response::PlainChat(message) => {
                    tracing::info!(?event, "processing event response",);

                    let response_chunks = split_message(&message, DISCORD_MAX_MESSAGE_LENGTH);

                    tracing::debug!(?response_chunks, "response chunks");

                    for chunk in response_chunks {
                        if let Err(error) = msg.channel_id.say(&ctx.http, chunk).await {
                            tracing::error!(%error, "error sending message");
                        }
                    }
                }
                Response::Bot(bot_message) => {
                    tracing::info!(?event, "processing bot message response",);

                    let message: String = bot_message.render_without_thinking_parts();

                    let response_chunks = split_message(&message, DISCORD_MAX_MESSAGE_LENGTH);

                    tracing::debug!(?response_chunks, "response chunks");

                    for chunk in response_chunks {
                        if let Err(error) = msg.channel_id.say(&ctx.http, chunk).await {
                            tracing::error!(%error, "error sending message");
                        }
                    }
                }
                Response::Ignored => {
                    tracing::debug!(?event, "consumer ignored the event",);
                }
            }
        }
    }
}

#[ext]
impl Message {
    fn mentions_ultron(&self) -> bool {
        self.mentions_user_id(ULTRON_USER_ID) || self.content.contains("<@&777660234842898483>")
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
    fn split_message_works() {
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
    fn split_markdown_syntax() {
        let message = "This is a **test** message with `code` and [link](https://example.com).";
        let max_length = 30;
        let chunks = split_message(message, max_length);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], "This is a **test** message");
        assert_eq!(chunks[1], "with `code` and");
        assert_eq!(chunks[2], "[link](https://example.com).");
    }
}

use bon::Builder;
use extend::ext;
use serde::Deserialize;
use serenity::{
    Client,
    all::{ChannelId, Context, EventHandler, GatewayIntents, Message, Typing, UserId},
    http::Http,
};
use std::{collections::HashMap, sync::Arc};
use tokio::task::JoinHandle;
use ultron_core::{
    Channel, Response, User,
    chatbot::{ChatBot, ChatInput},
    command::CommandParseError,
    dice::HELP_MESSAGE,
    event_processor::{Event, EventError, EventProcessor, EventType},
};

use crate::error::{DiscordBotError, DiscordBotResult};

mod error;

/// ultron#ultron-test
const DEFAULT_DEBUG_CHANNEL_ID: ChannelId = ChannelId::new(777725275856699402);
const DEFAULT_GENERAL_CHANNEL_ID: ChannelId = ChannelId::new(777658379212161077);
const DEFAULT_DND_CHANNEL_ID: ChannelId = ChannelId::new(874085144284258325);

/// FunZone#stream
const FUN_ZONE_STREAM_CHANNEL_ID: ChannelId = ChannelId::new(1375319100124827748);

/// FunZone#bots
const FUN_ZONE_BOT_CHANNEL_ID: ChannelId = ChannelId::new(1249097633520160808);

const CHANNELS: &[(Channel, ChannelId)] = &[
    (Channel::Debug, DEFAULT_DEBUG_CHANNEL_ID),
    (Channel::Psa, DEFAULT_GENERAL_CHANNEL_ID),
    (Channel::Dnd, DEFAULT_DND_CHANNEL_ID),
    (Channel::FunZoneBots, FUN_ZONE_BOT_CHANNEL_ID),
    (Channel::FunZoneStream, FUN_ZONE_STREAM_CHANNEL_ID),
];

/// the [`UserId`] of the bot itself
const ULTRON_USER_ID: UserId = UserId::new(777627943144652801);

/// the string used to mention the bot in messages
/// this is the bot's role mention, which is used in some servers.
/// TODO: expand on this
const ULTRON_USER_ID_STR: &str = "<@&777660234842898483>";

const DISCORD_MAX_MESSAGE_LENGTH: usize = 2000;

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

        let channels = Arc::new(Channels::new());

        let spawn_channels = channels.clone();
        let client_handle = tokio::spawn(async move {
            let mut client = Client::builder(&self.token, self.intents.0)
                .application_id(self.application_id.parse().unwrap())
                .event_handler(Handler {
                    event_processor: self.event_processor,
                    channels: spawn_channels,
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
            channels,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DiscordBot {
    http: Arc<Http>,
    client_handle: Arc<JoinHandle<DiscordBotResult<()>>>,
    channels: Arc<Channels>,
}

#[derive(Debug, Clone, Deserialize)]
struct Channels {
    by_id: HashMap<ChannelId, Channel>,
    by_name: HashMap<Channel, ChannelId>,
}

impl Channels {
    pub fn new() -> Self {
        let by_id = CHANNELS
            .iter()
            .map(|(channel, channel_id)| (*channel_id, *channel))
            .collect();

        let by_name = CHANNELS
            .iter()
            .map(|(channel, channel_id)| (*channel, *channel_id))
            .collect();

        Self { by_id, by_name }
    }

    pub fn by_id(&self, channel_id: &ChannelId) -> Option<&Channel> {
        self.by_id.get(channel_id)
    }

    pub fn by_name(&self, channel: &Channel) -> Option<&ChannelId> {
        self.by_name.get(channel)
    }
}

impl ChatBot for DiscordBot {
    type Error = DiscordBotError;

    async fn send_message(&self, channel: Channel, message: &str) -> DiscordBotResult<()> {
        let id: ChannelId = *self
            .channels
            .by_name(&channel)
            .ok_or(DiscordBotError::ChannelNotConfigured { channel })?;

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
    channels: Arc<Channels>,
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        tracing::debug!("handling message event: {:?}", msg);

        if let Err(error) = self.handle_message(ctx, msg).await {
            tracing::error!(%error, "error handling message");
        }
    }
}

impl Handler {
    async fn handle_message(&self, ctx: Context, msg: Message) -> DiscordBotResult<()> {
        let channel = self
            .channels
            .by_id(&msg.channel_id)
            .ok_or(DiscordBotError::ChannelNotRecognized { id: msg.channel_id })?;

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

        let chat_input = ChatInput::builder()
            .user(user)
            .content(msg.content.clone())
            .channel(*channel)
            .build();

        let event: Event = Event::new(chat_input, event_type)?;

        let results = Box::pin(self.event_processor.process(event.clone())).await;

        let results = match results {
            Ok(results) => results,
            Err(error) => {
                tracing::error!(
                    ?event,
                    %error,
                    "error processing event",
                );

                self.handle_event_error(&ctx, msg.channel_id, error).await?;

                return Ok(());
            }
        };

        tracing::debug!(?results, "processing event result");

        for result in results {
            self.handle_response(&ctx, msg.channel_id, result).await?;
        }

        Ok(())
    }

    async fn handle_event_error(
        &self,
        context: &Context,
        channel: ChannelId,
        error: EventError,
    ) -> DiscordBotResult<()> {
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
            channel.say(&context.http, error_message).await?;
        }

        Ok(())
    }

    async fn handle_response(
        &self,
        context: &Context,
        channel: ChannelId,
        response: Response,
    ) -> DiscordBotResult<()> {
        let response_chunks = match response {
            Response::PlainChat(message) => {
                tracing::info!("handling plain chat response: {message}");

                split_message(&message, DISCORD_MAX_MESSAGE_LENGTH)
            }
            Response::Bot(bot_message) => {
                let message: String = bot_message.render_without_thinking_parts();

                split_message(&message, DISCORD_MAX_MESSAGE_LENGTH)
            }
            Response::Ignored => {
                tracing::info!("response ignored");
                vec![]
            }
        };

        for chunk in response_chunks {
            channel.say(&context.http, chunk).await?;
        }

        Ok(())
    }
}

#[ext]
impl Message {
    fn mentions_ultron(&self) -> bool {
        self.mentions_user_id(ULTRON_USER_ID) || self.content.contains(ULTRON_USER_ID_STR)
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

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
            return;
        } else {
            User::from(msg.author.name.clone())
        };

        tracing::debug!(user = ?user, "message from user");

        let event_type: EventType = if msg.mentions_user_id(ULTRON_USER_ID) {
            tracing::info!(user = ?user, "message mentions bot, treating as natural language");
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

        let result = self.event_processor.process(event.clone()).await;

        tracing::debug!(?result, "processing event result");

        match result {
            Ok(Response::PlainChat(response)) => {
                tracing::info!(?event, "processing event response",);

                let bot_message: BotMessage =
                    ThinkingIterator::new(&response, "<think>", "</think>").collect();

                tracing::debug!(?bot_message, "bot message");

                // remove thinking parts
                let message: String = bot_message.render_without_thinking_parts();

                let response_chunks = split_message(&message, DISCORD_MAX_MESSAGE_LENGTH);

                tracing::debug!(?response_chunks, "response chunks");

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

/// TODO: move to core
#[derive(Debug, Clone)]
struct BotMessage {
    parts: Vec<MessagePart>,
}

impl FromIterator<MessagePart> for BotMessage {
    fn from_iter<I: IntoIterator<Item = MessagePart>>(iter: I) -> Self {
        let parts = iter.into_iter().collect();
        BotMessage { parts }
    }
}

impl BotMessage {
    pub fn render_without_thinking_parts(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| {
                if let MessagePart::Text(text) = part {
                    Some(text.clone())
                } else {
                    None // filter out thinking parts
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessagePart {
    Thinking(String),
    Text(String),
}

/// and iterator over a message that returns [`MessagePart`]s,
/// separating out different parts of the message,
/// crucially separating "thinking" sections from normal text.
pub struct ThinkingIterator<'msg> {
    message: &'msg str,
    start_delim: &'msg str,
    end_delim: &'msg str,
    cursor: usize,
}

impl<'msg> ThinkingIterator<'msg> {
    pub fn new(message: &'msg str, start_delim: &'msg str, end_delim: &'msg str) -> Self {
        Self {
            message,
            start_delim,
            end_delim,
            cursor: 0,
        }
    }
}

impl Iterator for ThinkingIterator<'_> {
    type Item = MessagePart;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor >= self.message.len() {
            return None;
        }

        split_next_thinking_section(
            &self.message[self.cursor..],
            self.start_delim,
            self.end_delim,
        )
        .map(|(first_section, thinking_section, _rest_of_message)| {
            if !first_section.is_empty() {
                self.cursor += first_section.len();
                MessagePart::Text(first_section.to_string())
            } else {
                self.cursor +=
                    thinking_section.len() + self.start_delim.len() + self.end_delim.len();
                MessagePart::Thinking(thinking_section.to_string())
            }
        })
        .or_else(|| {
            let rest = &self.message[self.cursor..];
            if !rest.is_empty() {
                self.cursor += rest.len();
                Some(MessagePart::Text(rest.to_string()))
            } else {
                None
            }
        })
    }
}

fn split_next_thinking_section<'msg>(
    message: &'msg str,
    start_delim: &str,
    end_delim: &str,
) -> Option<(&'msg str, &'msg str, &'msg str)> {
    message
        .find(start_delim)
        .and_then(|start_index| {
            message
                .find(end_delim)
                .map(|end_index_start| end_index_start + end_delim.len())
                .map(|end_index| (start_index, end_index))
        })
        .map(|(start_index, end_index)| {
            let first_section = &message[..start_index];
            let thinking_section =
                &message[start_index + start_delim.len()..end_index - end_delim.len()];
            let rest_of_message = &message[end_index..];

            Some((first_section, thinking_section, rest_of_message))
        })
        .unwrap_or(None)
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

    #[test]
    fn split_next_thinking_section_works() {
        let message = "This is a test <think>thinking part</think> and another part.";
        let start_delim = "<think>";
        let end_delim = "</think>";
        let result = split_next_thinking_section(message, start_delim, end_delim);
        assert!(result.is_some());
        let (first_section, thinking_section, rest_of_message) = result.unwrap();
        assert_eq!(first_section, "This is a test ");
        assert_eq!(thinking_section, "thinking part");
        assert_eq!(rest_of_message, " and another part.");
    }

    #[test]
    fn thinking_iterator_works() {
        let message = "This is a test <think>thinking part</think> and another part.";
        let start_delim = "<think>";
        let end_delim = "</think>";
        let mut iterator = ThinkingIterator::new(message, start_delim, end_delim);
        let first_part = iterator.next().unwrap();
        assert_eq!(first_part, MessagePart::Text("This is a test ".to_string()));
        let thinking_part = iterator.next().unwrap();
        assert_eq!(
            thinking_part,
            MessagePart::Thinking("thinking part".to_string())
        );
        let second_part = iterator.next().unwrap();
        assert_eq!(
            second_part,
            MessagePart::Text(" and another part.".to_string())
        );
        assert!(iterator.next().is_none());
    }

    #[test]
    fn thinking_iterator_handles_no_thinking() {
        let message = "This is a test message without thinking parts.";
        let start_delim = "<think>";
        let end_delim = "</think>";
        let mut iterator = ThinkingIterator::new(message, start_delim, end_delim);
        let first_part = iterator.next().unwrap();
        assert_eq!(first_part, MessagePart::Text(message.to_string()));
        assert!(iterator.next().is_none());
    }

    #[test]
    fn thinking_iterator_handles_multiple_thinking_sections() {
        let message = "This is a test <think>thinking part 1</think> and another <think>thinking part 2</think>.";
        let start_delim = "<think>";
        let end_delim = "</think>";
        let mut iterator = ThinkingIterator::new(message, start_delim, end_delim);
        let first_part = iterator.next().unwrap();
        assert_eq!(first_part, MessagePart::Text("This is a test ".to_string()));
        let thinking_part1 = iterator.next().unwrap();
        assert_eq!(
            thinking_part1,
            MessagePart::Thinking("thinking part 1".to_string())
        );
        let second_part = iterator.next().unwrap();
        assert_eq!(second_part, MessagePart::Text(" and another ".to_string()));
        let thinking_part2 = iterator.next().unwrap();
        assert_eq!(
            thinking_part2,
            MessagePart::Thinking("thinking part 2".to_string())
        );
        let last_part = iterator.next().unwrap();
        assert_eq!(last_part, MessagePart::Text(".".to_string()));
        assert!(iterator.next().is_none());
    }

    #[test]
    fn bot_message_render_without_thinking_parts() {
        let message = BotMessage {
            parts: vec![
                MessagePart::Text("This is a test".to_string()),
                MessagePart::Thinking("thinking part".to_string()),
                MessagePart::Text("and another part".to_string()),
            ],
        };
        let rendered = message.render_without_thinking_parts();
        assert_eq!(rendered, "This is a test\nand another part");
    }

    #[test]
    fn bot_message_render_without_thinking_parts_no_thinking_parts() {
        let message = BotMessage {
            parts: vec![MessagePart::Text("This is a test".to_string())],
        };
        let rendered = message.render_without_thinking_parts();
        assert_eq!(rendered, "This is a test");
    }
}

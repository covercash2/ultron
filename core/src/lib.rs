use std::{future::Future, marker::Send};

use command::{Command, CommandParseError};
use lm::{LanguageModel, LanguageModelError};
use serde::{Deserialize, Serialize};

pub mod command;
pub mod copypasta;
pub mod dice;
pub mod error;
pub mod http_server;
pub mod io;
pub mod lm;
pub mod ollama;

const DEFAULT_COMMAND_PREFIX: &str = "!ultron";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Debug,
    Psa,
    Dnd,
}

/// Input from a chat interface, such as Discord
#[derive(Debug, Clone, PartialEq)]
pub struct ChatInput {
    pub user: User,
    pub content: String,
}

impl ChatInput {
    pub fn anonymous(content: impl ToString) -> Self {
        Self {
            user: User::Anonymous,
            content: content.to_string(),
        }
    }

    pub fn strip_prefix(&self) -> Result<&str, CommandParseError> {
        self.content
            .strip_prefix(DEFAULT_COMMAND_PREFIX)
            .map(|content| content.trim())
            .ok_or(CommandParseError::MissingPrefix(self.content.clone()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Command,
    NaturalLanguage,
}

/// represents an event that can be processed by the bot.
/// stripped of any command prefix or control characters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Event {
    user: User,
    content: String,
    event_type: EventType,
}

impl Event {
    /// Creates a new event from a chat input and an event type.
    /// If the event type is `Command`, it will strip the command prefix from the content.
    pub fn new(chat_input: ChatInput, event_type: EventType) -> Result<Self, CommandParseError> {
        let content = if event_type == EventType::Command {
            chat_input.strip_prefix()?.to_string()
        } else {
            chat_input.content.clone()
        };

        Ok(Event {
            user: chat_input.user,
            content,
            event_type,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    PlainChat(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum User {
    Ultron,
    Anonymous,
    #[strum(serialize = "{0}")]
    Normal(String),
}

impl<T: Into<String>> From<T> for User {
    fn from(name: T) -> Self {
        let name = name.into();
        match name.as_str() {
            "ultron" | "Ultron" => User::Ultron,
            _ => User::Normal(name),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("failed to parse command from input: {0}")]
    CommandParse(#[from] CommandParseError),
    #[error("failed to parse dice roll from input: {0}")]
    DiceRollParse(#[from] dice::DiceRollError),

    #[error("language model error: {0}")]
    LanguageModel(#[from] lm::LanguageModelError),
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Default))]
pub struct EventProcessor {
    language_model: LanguageModel,
    raw_events: Vec<Event>,
}

impl EventProcessor {
    pub fn new(lm_endpoint: impl AsRef<str>) -> Result<Self, LanguageModelError> {
        let language_model = LanguageModel::ollama(lm_endpoint.as_ref(), Default::default())?;

        let system_message = Event {
            user: User::Ultron,
            content: "You are Ultron, a helpful AI assistant. Respond to commands and natural language inputs.".to_string(),
            event_type: EventType::NaturalLanguage,
        };

        let raw_events = vec![system_message];

        Ok(Self {
            language_model,
            raw_events,
        })
    }
}

impl EventProcessor {
    pub async fn process(&self, event: impl Into<Event>) -> Result<Response, EventError> {
        let event = event.into();
        tracing::debug!(?event, "processing event");

        match event.event_type {
            EventType::Command => {
                let command: Command = event.try_into()?;
                let output = command.execute()?;

                tracing::debug!("computed output: {}", output);

                Ok(Response::PlainChat(output))
            }
            EventType::NaturalLanguage => {
                let events = self
                    .raw_events
                    .iter()
                    .cloned()
                    .chain(std::iter::once(event))
                    .collect::<Vec<_>>();

                let next_event = self.language_model.chat(events).await?;

                tracing::info!(
                    user = ?next_event.user,
                    "language model response"
                );

                Ok(Response::PlainChat(next_event.content))
            }
        }
    }
}

pub trait ChatBot: Clone + Send + Sync {
    type Error: std::error::Error;

    fn send_message(
        &self,
        channel: Channel,
        message: &str,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn debug(
        &self,
        message: &str,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        async { self.send_message(Channel::Debug, message).await }
    }

    fn psa(
        &self,
        message: &str,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        async { self.send_message(Channel::Psa, message).await }
    }

    fn dnd(
        &self,
        message: &str,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        async { self.send_message(Channel::Dnd, message).await }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let event: ChatInput = ChatInput::anonymous("!ultron echo hello");
        let event: Event =
            Event::new(event, EventType::Command).expect("should parse chat input to event");
        let processor = EventProcessor::default();
        let response = processor
            .process(event)
            .await
            .expect("echo should not error");
        assert_eq!(response, Response::PlainChat("hello".to_string()));
    }

    #[test]
    fn strip_prefix() {
        let chat_input: ChatInput = ChatInput::anonymous("!ultron hello");
        let input: Event = Event::new(chat_input, EventType::Command)
            .expect("should parse chat input to api input");
        assert_eq!(input.user, User::Anonymous);
        assert_eq!(input.content, "hello");
    }

    #[test]
    fn user_name_display_snapshots() {
        let user = User::Normal("test_user".to_string());
        assert_eq!(user.to_string(), "test_user");

        let user = User::Ultron;
        assert_eq!(user.to_string(), "ultron");

        let user = User::Anonymous;
        assert_eq!(user.to_string(), "anonymous");
    }
}

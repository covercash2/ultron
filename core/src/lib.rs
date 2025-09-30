use std::{future::Future, marker::Send};

use command::{Command, CommandParseError};
use serde::{Deserialize, Serialize};

pub mod command;
pub mod copypasta;
pub mod dice;
pub mod error;
pub mod http_server;
pub mod io;

const DEFAULT_COMMAND_PREFIX: &str = "!ultron";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Debug,
    Psa,
    Dnd,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to parse command from input: {0:?}")]
    CommandParse(CommandParseError),
}

/// Input from a chat interface, such as Discord
#[derive(Debug, Clone, PartialEq)]
pub struct ChatInput {
    pub user: String,
    pub content: String,
}

impl ChatInput {
    pub fn anonymous(content: impl ToString) -> Self {
        Self {
            user: "anonymous".to_string(),
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

impl TryFrom<ChatInput> for Event {
    type Error = CommandParseError;

    fn try_from(chat_input: ChatInput) -> Result<Self, Self::Error> {
        let content = chat_input.strip_prefix()?.to_string();
        Ok(Event {
            user: chat_input.user,
            content,
        })
    }
}

/// represents an event that can be processed by the bot.
/// stripped of any command prefix or control characters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Event {
    user: String,
    content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    PlainChat(String),
}

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("failed to parse command from input: {0}")]
    CommandParse(#[from] CommandParseError),
    #[error("failed to parse dice roll from input: {0}")]
    DiceRollParse(#[from] dice::DiceRollError),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EventProcessor {
    raw_events: Vec<Event>,
}

impl EventProcessor {
    pub async fn process(&self, event: impl Into<Event>) -> Result<Response, EventError> {
        let event = event.into();
        tracing::debug!(?event, "processing event");

        let command: Command = event.try_into()?;
        let output = command.execute()?;

        tracing::debug!("computed output: {}", output);

        Ok(Response::PlainChat(output))
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
        let event: Event = event.try_into().expect("should parse chat input to event");
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
        let input: Event = chat_input
            .try_into()
            .expect("should parse chat input to api input");
        assert_eq!(input.user, "anonymous");
        assert_eq!(input.content, "hello");
    }
}

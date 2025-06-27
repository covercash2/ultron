use std::{future::Future, marker::Send, ops::Deref};

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

#[derive(Debug, Clone, PartialEq)]
pub struct ChatInput {
    content: String,
}

impl ChatInput {
    pub fn strip_prefix(&self) -> Result<&str, CommandParseError> {
        self.content
            .strip_prefix(DEFAULT_COMMAND_PREFIX)
            .map(|content| content.trim())
            .ok_or(CommandParseError::MissingPrefix(self.content.clone()))
    }
}

impl TryFrom<ChatInput> for ApiInput {
    type Error = CommandParseError;

    fn try_from(chat_input: ChatInput) -> Result<Self, Self::Error> {
        let content = chat_input.strip_prefix()?;
        Ok(ApiInput(content.to_string()))
    }
}

impl<T> From<T> for ChatInput
where
    T: ToString,
{
    fn from(content: T) -> Self {
        ChatInput {
            content: content.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    ChatInput(ChatInput),
    ApiInput(ApiInput),
}

impl TryFrom<Event> for ApiInput {
    type Error = CommandParseError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::ChatInput(chat_input) => chat_input.try_into(),
            Event::ApiInput(api_input) => Ok(api_input),
        }
    }
}

impl<T: AsRef<str>> From<T> for ApiInput {
    fn from(input: T) -> Self {
        ApiInput(input.as_ref().to_string())
    }
}

impl From<ApiInput> for Event {
    fn from(api_input: ApiInput) -> Self {
        Event::ApiInput(api_input)
    }
}

/// the base type for all inputs to the bot.
/// other input types resolve to this type
/// with [`From`] (or [`TryFrom`]) implementations.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiInput(String);

impl Deref for ApiInput {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ChatInput> for Event {
    fn from(chat_input: ChatInput) -> Self {
        Event::ChatInput(chat_input)
    }
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

#[derive(Debug, Clone)]
pub struct EventProcessor;

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
        let event: ChatInput = "!ultron echo hello".into();
        let event: Event = Event::ChatInput(event);
        let processor = EventProcessor;
        let response = processor
            .process(event)
            .await
            .expect("echo should not error");
        assert_eq!(response, Response::PlainChat("you said: hello".to_string()));
    }

    #[test]
    fn strip_prefix() {
        let chat_input: ChatInput = "!ultron hello".into();
        let prefix = chat_input.strip_prefix().unwrap();
        assert_eq!(prefix, "hello");
    }
}

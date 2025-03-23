use std::{future::Future, marker::Send, ops::Deref, str::FromStr};

use dice::{DiceRoll, HELP_MESSAGE};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumMessage, IntoEnumIterator};

pub mod dice;
pub mod http_server;

const DEFAULT_COMMAND_PREFIX: &str = "!ultron";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
        tracing::debug!("processing event: {:?}", event);

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

#[derive(thiserror::Error, Debug, PartialEq, Clone)]
pub enum CommandParseError {
    #[error("input is missing prefix {0}")]
    MissingPrefix(String),
    #[error("input is missing command {0}")]
    MissingCommand(String),
    #[error("undefined command in input {0}")]
    UndefinedCommand(String),
}

#[derive(Debug, Clone, PartialEq, strum::EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter, Display, EnumMessage))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
pub enum Command {
    #[strum_discriminants(strum(message = "make Ultron say something"))]
    Echo(String),
    #[strum_discriminants(strum(message = "roll some dice"))]
    Roll(String),
    #[strum_discriminants(strum(message = "get help"))]
    Help,
}

impl Command {
    pub fn execute(self) -> Result<String, EventError> {
        let result = match self {
            Command::Echo(message) => message.to_string(),
            Command::Roll(input) => {
                let dice_roll: DiceRoll = input.parse()?;
                dice_roll.to_string()
            }
            Command::Help => CommandDiscriminants::iter().fold(String::new(), |acc, command| {
                format!(
                    "{}\n✨`{}` 👉 {}",
                    acc,
                    command,
                    command.get_message().unwrap_or("oops no message")
                )
            }),
        };
        Ok(result)
    }
}

impl TryFrom<Event> for Command {
    type Error = CommandParseError;

    fn try_from(input: Event) -> Result<Self, Self::Error> {
        let input: ApiInput = input.try_into()?;

        input.parse()
    }
}

impl FromStr for Command {
    type Err = CommandParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut iterator = input.split_whitespace();

        let command = iterator
            .next()
            .ok_or(CommandParseError::MissingCommand(input.to_string()))?;

        // the rest of the input joined by spaces
        let rest = iterator.collect::<Vec<_>>().join(" ");

        match command {
            "echo" => Ok(Command::Echo(rest.to_string())),
            "roll" => Ok(Command::Roll(rest.to_string())),
            "help" => Ok(Command::Help),
            _ => Err(CommandParseError::UndefinedCommand(command.to_string())),
        }
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
    fn command_parse() {
        let command: Command = "echo hello".parse().unwrap();
        assert_eq!(command, Command::Echo("hello".to_string()));
    }

    #[test]
    fn command_parse_missing_command() {
        let command: Result<Command, CommandParseError> = "".parse();
        assert_eq!(
            command.expect_err("should fail to parse"),
            CommandParseError::MissingCommand("".to_string())
        );
    }

    #[test]
    fn command_parse_undefined_command() {
        let command: Result<Command, CommandParseError> = "undefined hello".parse();
        assert_eq!(
            command.expect_err("should fail to parse"),
            CommandParseError::UndefinedCommand("undefined hello".to_string())
        );
    }

    #[test]
    fn strip_prefix() {
        let chat_input: ChatInput = "!ultron hello".into();
        let prefix = chat_input.strip_prefix().unwrap();
        assert_eq!(prefix, "hello");
    }
}

use std::{future::Future, marker::Send};

use command::CommandParseError;
use event_processor::BotMessage;
use serde::{Deserialize, Serialize};

pub mod command;
pub mod copypasta;
pub mod dice;
pub mod error;
pub mod event_processor;
pub mod http_server;
pub mod io;
pub mod lm;
pub mod ollama;
pub mod mcp;

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

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    PlainChat(String),
    Bot(BotMessage),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum User {
    Ultron,
    System,
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

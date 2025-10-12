use nlp::response::BotMessage;
use serde::{Deserialize, Serialize};

pub mod chatbot;
pub mod command;
pub mod copypasta;
pub mod dice;
pub mod error;
pub mod event_processor;
pub mod http_server;
pub mod io;
pub mod mcp;
pub mod nlp;

const DEFAULT_COMMAND_PREFIX: &str = "!ultron";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Debug,
    Psa,
    Dnd,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// a plain chat response, e.g. from a command
    PlainChat(String),
    /// a response from the bot, e.g. from an LLM
    /// that may contain additional metadata.
    /// see [`event_processor::BotMessage`].
    Bot(BotMessage),
    /// the [`crate::event_processor::EventConsumer`] ignored the event
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum User {
    /// the assistant role in an LLM
    Ultron,
    /// the system role in an LLM
    System,
    /// a user role that is not identified
    Anonymous,
    /// a user role with a name
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

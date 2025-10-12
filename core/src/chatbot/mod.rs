use crate::{command::CommandParseError, Channel, User, DEFAULT_COMMAND_PREFIX};

pub trait ChatBot: Clone + Send + Sync {
    type Error: Into<crate::error::Error>;

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


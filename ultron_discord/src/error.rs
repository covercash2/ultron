use serenity::all::ChannelId;

pub type DiscordBotResult<T> = Result<T, DiscordBotError>;

#[derive(Debug, thiserror::Error)]
pub enum DiscordBotError {
    #[error("channel not configured, `{channel}`")]
    ChannelNotConfigured { channel: ultron_core::Channel },

    #[error("channel not recognized, `{id}`")]
    ChannelNotRecognized { id: ChannelId },

    #[error(transparent)]
    CommandParse(#[from] ultron_core::command::CommandParseError),

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

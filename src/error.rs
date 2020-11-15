use std::env::VarError;

use serenity::Error as DiscordError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub enum Error {
    DiscordError(DiscordError),
    BadApiKey(VarError),
    UnknownCommand(String),
}

impl From<DiscordError> for Error {
    fn from(err: DiscordError) -> Self {
	Error::DiscordError(err)
    }
}

impl From<VarError> for Error {
    fn from(v: VarError) -> Self {
	Error::BadApiKey(v)
    }
}

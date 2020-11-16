use std::env::VarError;

use hubcaps::Error as GithubError;

use serenity::Error as DiscordError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    DiscordError(DiscordError),
    GithubError(GithubError),
    BadApiKey(VarError),
    UnknownCommand(String),
    Unexpected(String),
}

impl From<DiscordError> for Error {
    fn from(err: DiscordError) -> Self {
	Error::DiscordError(err)
    }
}

impl From<GithubError> for Error {
    fn from(err: GithubError) -> Self {
	Error::GithubError(err)
    }
}

impl From<VarError> for Error {
    fn from(v: VarError) -> Self {
	Error::BadApiKey(v)
    }
}

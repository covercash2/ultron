use std::{env::VarError, io::Error as IoError};

use serde_json::Error as JsonError;

use hubcaps::Error as GithubError;

use serenity::Error as DiscordError;
use tokio::sync::mpsc::error::SendError;

use db::error::Error as DbError;

use crate::coins::Transaction;
use crate::gambling::Error as GambleError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Db(DbError),
    DiscordError(DiscordError),
    GithubError(GithubError),
    BadApiKey(VarError),
    Json(JsonError),
    TransactionSend(SendError<Transaction>),
    TransactionReceipt,
    TransactionFailed(String),
    Io(IoError),
    UnknownCommand(String),
    CommandParse(String),
    ReceiptProcess(String),
    GambleError(GambleError),
    MessageBuild(String),
    Unknown(String),
}

impl From<DbError> for Error {
    fn from(err: DbError) -> Self {
        Error::Db(err)
    }
}

impl From<SendError<Transaction>> for Error {
    fn from(err: SendError<Transaction>) -> Self {
        Error::TransactionSend(err)
    }
}

impl From<JsonError> for Error {
    fn from(err: JsonError) -> Self {
        Error::Json(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
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

impl From<GambleError> for Error {
    fn from(err: GambleError) -> Self {
        Error::GambleError(err)
    }
}

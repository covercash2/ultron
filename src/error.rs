use std::{env::VarError, io::Error as IoError};

use hubcaps::Error as GithubError;
use reqwest::Error as ReqwestError;
use serde_json::Error as JsonError;
use serenity::Error as DiscordError;

use tokio::sync::mpsc::error::SendError;

use crate::coins::Transaction;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    DiscordError(DiscordError),
    GithubError(GithubError),
    HnApiReqwestError(ReqwestError),
    BadApiKey(VarError),
    Json(JsonError),
    TransactionSend(SendError<Transaction>),
    TransactionReceipt,
    TransactionFailed(String),
    Io(IoError),
    UnknownCommand(String),
    CommandParse(String),
    CommandProcess(String),
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

impl From<ReqwestError> for Error {
    fn from(v: VarError) -> Self {
        Error::BadApiKey(v)
    }
}

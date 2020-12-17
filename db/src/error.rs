use std::{num::ParseIntError, env::VarError};

use diesel::result::Error as DieselError;
use diesel::ConnectionError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    CoinOverflow,
    NotFound(String),
    Unexpected(String),
    Db(DieselError),
    IdParse(ParseIntError),
    Env(VarError),
    Connection(ConnectionError)
}

impl From<DieselError> for Error {
    fn from(err: DieselError) -> Error {
	Error::Db(err)
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Error {
	Error::IdParse(err)
    }
}

impl From<VarError> for Error {
    fn from(err: VarError) -> Error {
	Error::Env(err)
    }
}

impl From<ConnectionError> for Error {
    fn from(err: ConnectionError) -> Error {
	Error::Connection(err)
    }
}

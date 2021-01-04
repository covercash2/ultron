use std::{env::VarError, num::{ParseIntError, TryFromIntError}};

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
    BadId(TryFromIntError),
    Env(VarError),
    Connection(ConnectionError),
    RecordExists,
    InsufficientFunds,
}

impl From<TryFromIntError> for Error {
    fn from(err: TryFromIntError) -> Error {
	Error::BadId(err)
    }
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

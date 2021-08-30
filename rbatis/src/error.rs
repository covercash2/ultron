use rbatis::Error as RbError;

use std::num::ParseIntError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Rbatis(RbError),
    ParseColumn(ParseIntError),
    Schema(&'static str),
    CoinOverflow(i64),
}

impl Into<Error> for RbError {
    fn into(self) -> Error {
        Error::Rbatis(self)
    }
}

impl Into<Error> for ParseIntError {
    fn into(self) -> Error {
        Error::ParseColumn(self)
    }
}

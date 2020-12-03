#[macro_use]
extern crate diesel;
use diesel::{prelude::*, sqlite::SqliteConnection};

mod error;
pub mod model;
pub mod schema;

use schema::bank_accounts::dsl::*;
use model::BankAccount;
use error::*;

pub struct Db {
    connection: SqliteConnection,
}

impl Db {
    pub fn open(database_url: &str) -> Result<Db> {
        let connection = establish_connection(database_url)?;

        Ok(Db { connection })
    }

    pub fn show_accounts(&self) -> Result<Vec<BankAccount>> {
        bank_accounts.load::<BankAccount>(&self.connection)
            .map_err(Into::into)
    }
}

fn establish_connection(database_url: &str) -> Result<SqliteConnection> {
    SqliteConnection::establish(&database_url).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

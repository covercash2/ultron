#[macro_use]
extern crate diesel;
use diesel::{prelude::*, sqlite::SqliteConnection};

mod error;
pub mod model;
pub mod schema;

use error::*;
use model::{BankAccount, ChannelUser};
use schema::bank_accounts::dsl::*;

pub struct Db {
    connection: SqliteConnection,
}

impl Db {
    pub fn open(database_url: &str) -> Result<Db> {
        let connection = establish_connection(database_url)?;

        Ok(Db { connection })
    }

    pub fn show_accounts(&self) -> Result<Vec<BankAccount>> {
        bank_accounts
            .load::<BankAccount>(&self.connection)
            .map_err(Into::into)
    }

    pub fn insert_bank_account(&self, server: &u64, user: &u64, amount: &i32) -> Result<usize> {
        diesel::insert_into(schema::bank_accounts::table)
            .values(&BankAccount::new(server, user, amount))
            .execute(&self.connection)
            .map_err(Into::into)
    }

    pub fn update_balance(&self, server: &u64, user: &u64, new_balance: &i32) -> Result<usize> {
        let server = server.to_string();
        let user = user.to_string();
        diesel::update(bank_accounts.find((server, user)))
            .set(balance.eq(new_balance))
            .execute(&self.connection)
            .map_err(Into::into)
    }

    pub fn show_channel_users(&self) -> Result<Vec<ChannelUser>> {
        use schema::channel_users::dsl::*;
        channel_users
            .load::<ChannelUser>(&self.connection)
            .map_err(Into::into)
    }

    pub fn channel_users(&self, server: &u64, channel: &u64) -> Result<Vec<ChannelUser>> {
        use schema::channel_users::dsl::*;
        let server = server.to_string();
        let channel = channel.to_string();
        channel_users
            .filter(server_id.eq(server))
            .filter(channel_id.eq(channel))
            .load::<ChannelUser>(&self.connection)
            .map_err(Into::into)
    }

    pub fn insert_channel_user(&self, server: &u64, channel: &u64, user: &u64) -> Result<usize> {
        diesel::insert_into(schema::channel_users::table)
            .values(&ChannelUser::new(server, channel, user))
            .execute(&self.connection)
            .map_err(Into::into)
    }

    pub fn channel_user_balances(&self, server: &u64, channel: &u64) -> Result<Vec<BankAccount>> {
        use schema::channel_users::dsl::*;
        let server = server.to_string();
        let channel = channel.to_string();

        let user_ids = channel_users
            .select(user_id)
            .filter(server_id.eq(server))
            .filter(channel_id.eq(channel));

        bank_accounts
            .filter(schema::bank_accounts::dsl::user_id.eq_any(user_ids))
            .load::<BankAccount>(&self.connection)
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

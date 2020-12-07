#[macro_use]
extern crate diesel;
use diesel::{prelude::*, sqlite::SqliteConnection};

use std::{convert::TryInto, fmt};

pub mod error;
pub mod model;
pub mod schema;

use error::*;
use model::{BankAccount, ChannelUser};
use schema::bank_accounts::dsl::*;

pub struct Db {
    connection: SqliteConnection,
}

impl fmt::Debug for Db {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Db").finish()
    }
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

    pub fn user_account(&self, server: &u64, user: &u64) -> Result<BankAccount> {
        let server = server.to_string();
        let user = user.to_string();
        bank_accounts
            .find((&server, &user))
            .load::<BankAccount>(&self.connection)
            .map_err(Into::into)
            .and_then(|vec| {
                match vec.len() {
                    1 => Ok(vec[0].clone()), // return the only value
                    0 => Err(Error::NotFound(format!(
                        "unable to find user account: #s{} #u{}",
                        server, user
                    ))),
                    _ => Err(Error::Unexpected("too many records returned".to_owned())),
                }
            })
    }

    pub fn user_accounts(&self, server: &u64, users: &[u64]) -> Result<Vec<BankAccount>> {
        let server = server.to_string();
        let users: Vec<String> = users.iter().map(|u| u.to_string()).collect();
        bank_accounts
            .filter(server_id.eq(server))
            .filter(user_id.eq_any(users))
            .load::<BankAccount>(&self.connection)
            .map_err(Into::into)
    }

    pub fn insert_bank_account(&self, server: &u64, user: &u64, amount: &i64) -> Result<usize> {
        let amount: i32 = (*amount).try_into().map_err(|_e| Error::CoinOverflow)?;
        diesel::insert_into(schema::bank_accounts::table)
            .values(&BankAccount::new(server, user, &amount))
            .execute(&self.connection)
            .map_err(Into::into)
    }

    pub fn update_balance(&self, server: &u64, user: &u64, new_balance: &i64) -> Result<usize> {
        let server = server.to_string();
        let user = user.to_string();
        let new_balance: i32 = (*new_balance)
            .try_into()
            .map_err(|_e| Error::CoinOverflow)?;
        diesel::update(bank_accounts.find((server, user)))
            .set(balance.eq(new_balance))
            .execute(&self.connection)
            .map_err(Into::into)
    }

    pub fn increment_balance(&self, server: &u64, user: &u64, amount: &i64) -> Result<usize> {
        let server_s = server.to_string();
        let user_s = user.to_string();
        let amount: i32 = (*amount).try_into().map_err(|_e| Error::CoinOverflow)?;
        let current_balance: i32 = bank_accounts
            .select(balance)
            .find((server_s, user_s))
            .first::<i32>(&self.connection)?;
        let new_balance: i32 = current_balance + amount;

        self.update_balance(server, user, &new_balance.into())
    }

    pub fn tip(&self, server: &u64, from_user: &u64, to_user: &u64) -> Result<usize> {
	let from_amount: i32 = 1;
	let from_account = BankAccount::new(server, from_user, &from_amount);

	let to_amount: i32 = 2;
	let to_account = BankAccount::new(server, to_user, &2);

	self.connection.transaction::<_, Error, _>(|| {
	    let mut num_records = diesel::insert_into(bank_accounts)
		.values(&from_account)
		.on_conflict((server_id, user_id))
		.do_update()
		.set(balance.eq(balance + from_amount))
		.execute(&self.connection)?;

	    num_records += diesel::insert_into(bank_accounts)
		.values(&to_account)
		.on_conflict((server_id, user_id))
		.do_update()
		.set(balance.eq(balance + to_amount))
		.execute(&self.connection)?;

	    Ok(num_records)
	})
    }

    pub fn untip(&self, server: &u64, from_user: &u64, to_user: &u64) -> Result<usize> {
	let from_amount: i32 = -1;
	let from_account = BankAccount::new(server, from_user, &from_amount);

	let to_amount: i32 = -2;
	let to_account = BankAccount::new(server, to_user, &2);

	self.connection.transaction::<_, Error, _>(|| {
	    let mut num_records = diesel::insert_into(bank_accounts)
		.values(&from_account)
		.on_conflict((server_id, user_id))
		.do_update()
		.set(balance.eq(balance + from_amount))
		.execute(&self.connection)?;

	    num_records += diesel::insert_into(bank_accounts)
		.values(&to_account)
		.on_conflict((server_id, user_id))
		.do_update()
		.set(balance.eq(balance + to_amount))
		.execute(&self.connection)?;

	    Ok(num_records)
	})
    }

    pub fn transfer_coins(
        &self,
        server: &u64,
        from_user: &u64,
        to_user: &u64,
        amount: &i64,
    ) -> Result<usize> {
	let to_amount: i32 = (*amount).try_into()
	    .map_err(|_e| Error::CoinOverflow)?;
	let from_amount: i32 = -to_amount;

	let from_account = BankAccount::new(server, from_user, &from_amount);
	let to_account = BankAccount::new(server, to_user, &to_amount);

	self.connection.transaction::<_, Error, _>(|| {
	    let mut record_num = diesel::insert_into(bank_accounts)
		.values(&from_account)
		.on_conflict((server_id, user_id))
		.do_update()
		.set(balance.eq(balance + from_amount))
		.execute(&self.connection)?;

	    record_num += diesel::insert_into(bank_accounts)
		.values(&to_account)
		.on_conflict((server_id, user_id))
		.do_update()
		.set(balance.eq(balance + to_amount))
		.execute(&self.connection)?;

	    Ok(record_num)
	})
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
            .filter(channel_id.eq(channel).and(server_id.eq(&server)));

        bank_accounts
            .filter(schema::bank_accounts::dsl::user_id.eq_any(user_ids))
            .filter(schema::bank_accounts::dsl::server_id.eq(&server))
            .load::<BankAccount>(&self.connection)
            .map_err(Into::into)
    }

    pub fn log_user(&self, server: &u64, channel: &u64, user: &u64) -> Result<usize> {
	diesel::insert_or_ignore_into(schema::channel_users::table)
	    .values(&ChannelUser::new(server, channel, user))
	    .execute(&self.connection)
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

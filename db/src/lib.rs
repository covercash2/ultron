#[macro_use]
extern crate diesel;
use diesel::{
    prelude::*,
    sqlite::{Sqlite, SqliteConnection},
};

use log::*;
use std::{convert::TryInto, fmt};

mod schema;

mod accounts;
mod ids;
mod inventory;
mod items;

pub mod error;
pub mod model;

use error::*;
use model::{BankAccount, ChannelUser, InventoryItem, Item, Optout, UpdateItem};
use schema::bank_accounts::dsl::*;

pub use accounts::TransferResult;

type Backend = Sqlite;

pub struct Db {
    connection: SqliteConnection,
}

impl fmt::Debug for Db {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Db").finish()
    }
}

impl Db {
    pub fn open(database_url: impl AsRef<str>) -> Result<Db> {
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

    /// returns number of records changed
    pub fn increment_balance(&self, server: &u64, user: &u64, amount: &i64) -> Result<usize> {
        let amount: i32 = (*amount).try_into().map_err(|_e| Error::CoinOverflow)?;

        let account = BankAccount::new(server, user, &amount);

        diesel::insert_into(bank_accounts)
            .values(&account)
            .on_conflict((server_id, user_id))
            .do_update()
            .set(balance.eq(balance + amount))
            .execute(&self.connection)
            .map_err(Into::into)
    }

    pub fn tip(&self, server: &u64, from_user: &u64, to_user: &u64) -> Result<()> {
        if from_user == to_user {
            return Err(Error::SelfTip);
        }

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

            if num_records == 2 {
                Ok(())
            } else {
                Err(Error::Unexpected(format!(
                    "unexpected number of records affected: {}",
                    num_records
                )))
            }
        })
    }

    pub fn untip(&self, server: &u64, from_user: &u64, to_user: &u64) -> Result<()> {
        if from_user == to_user {
            return Err(Error::SelfTip);
        }

        let from_amount: i32 = -1;
        let from_account = BankAccount::new(server, from_user, &from_amount);

        let to_amount: i32 = -2;
        let to_account = BankAccount::new(server, to_user, &2);

        self.connection.transaction::<_, Error, _>(|| {
            // TODO it doesn't really make sense to do `on_conflict` here
            // it should just error out if the accounts don't exist
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

            if num_records == 2 {
                Ok(())
            } else {
                Err(Error::Unexpected(format!(
                    "unexpected number of records affected: {}",
                    num_records
                )))
            }
        })
    }

    pub fn transfer_coins(
        &self,
        server: &u64,
        from_user: &u64,
        to_user: &u64,
        amount: &i64,
    ) -> Result<TransferResult> {
        accounts::transfer_coins(&self.connection, server, from_user, to_user, amount)
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

    pub fn all_items(&self) -> Result<Vec<Item>> {
        items::show_all(&self.connection)
    }

    pub fn create_item(&self, item: Item) -> Result<()> {
        items::create(&self.connection, item)
    }

    pub fn update_item(&self, item: UpdateItem) -> Result<()> {
        items::update(&self.connection, item)
    }

    pub fn dump_inventory(&self) -> Result<Vec<InventoryItem>> {
        inventory::show_all(&self.connection)
    }

    /// returns Error::RecordExists if the item already exists
    pub fn add_inventory_item(&self, inventory_item: InventoryItem) -> Result<()> {
        let server = inventory_item.server_id()?.to_string();
        let user = inventory_item.user_id()?.to_string();

        let num_records = self.connection.transaction(|| {
            let price = schema::items::dsl::items
                .find(&inventory_item.item_id)
                .select(schema::items::dsl::price)
                .first::<i32>(&self.connection)?;
            let account = schema::bank_accounts::dsl::bank_accounts.find((&server, &user));
            let account_balance = account
                .select(schema::bank_accounts::dsl::balance)
                .first::<i32>(&self.connection)?;

            if account_balance > price {
                inventory::add_item(&self.connection, inventory_item)?;
                diesel::update(account)
                    .set(
                        schema::bank_accounts::dsl::balance
                            .eq(schema::bank_accounts::dsl::balance - price),
                    )
                    .execute(&self.connection)
                    .map_err(Into::into)
            } else {
                Err(Error::InsufficientFunds)
            }
        })?;

        match num_records {
            0 => Err(Error::RecordExists),
            1 => {
                // added item successfully
                Ok(())
            }
            _ => {
                // unknown error
                Err(Error::Unexpected(format!(
                    "unexpected number of records changed: {}",
                    num_records
                )))
            }
        }
    }

    pub fn item(&self, item_id: &i32) -> Result<Item> {
        items::get(&self.connection, item_id)
    }

    pub fn user_inventory(&self, server: u64, user: u64) -> Result<Vec<Item>> {
        let server = server.to_string();
        let user = user.to_string();
        inventory::user_inventory(&self.connection, server, user)
    }

    pub fn user_has_item(&self, server: u64, user: u64, item: u64) -> Result<bool> {
        let server = server.to_string();
        let user = user.to_string();
        let item: i32 = item.try_into()?;
        inventory::user_has_item(&self.connection, server, user, item)
    }

    pub fn delete_inventory_item(&self, inventory_item: InventoryItem) -> Result<()> {
        inventory::delete_item(&self.connection, inventory_item).and_then(|num_records| {
            match num_records {
                0 => Err(Error::NotFound("no record found to delete".to_owned())),
                1 => Ok(()),
                n => Err(Error::Unexpected(format!(
                    "unexpected number of records returned:{}",
                    n
                ))),
            }
        })
    }

    pub fn optout(&self, server: u64, user: u64) -> Result<()> {
        use schema::optouts;

        diesel::insert_into(optouts::table)
            .values(Optout::new(&server, &user))
            .execute(&self.connection)
            .map_err(Into::into)
            .and_then(|num_records| match num_records {
                0 => {
                    info!("user has already opt out");
                    Ok(())
                }
                1 => {
                    info!("user has opt out");
                    Ok(())
                }
                n => {
                    warn!("unexpected number of records altered: {}", n);
                    Err(Error::Unexpected(format!(
                        "unexpected number of records altered: {}",
                        n
                    )))
                }
            })
    }

    pub fn optin(&self, server: u64, user: u64) -> Result<()> {
        use schema::optouts::dsl::*;

        let server = server.to_string();
        let user = user.to_string();

        let optin = optouts.find((&server, &user));

        diesel::delete(optin)
            .execute(&self.connection)
            .map_err(Into::into)
            .and_then(|num_records| match num_records {
                0 => {
                    info!("user has not opt out");
                    Ok(())
                }
                1 => {
                    info!("user has opt in");
                    Ok(())
                }
                n => {
                    warn!("unexpected number of records altered: {}", n);
                    Err(Error::Unexpected(format!(
                        "unexpected number of records altered: {}",
                        n
                    )))
                }
            })
    }

    pub fn all_optouts(&self) -> Result<Vec<Optout>> {
        use schema::optouts::dsl::*;
        optouts.load::<Optout>(&self.connection).map_err(Into::into)
    }
}

fn establish_connection(database_url: impl AsRef<str>) -> Result<SqliteConnection> {
    SqliteConnection::establish(database_url.as_ref()).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

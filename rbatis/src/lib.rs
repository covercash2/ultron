#[macro_use]
extern crate rbatis;

use std::{convert::TryInto, fmt::Debug, future::Future, num::ParseIntError};

use rbatis::{crud::CRUD, rbatis::Rbatis, wrapper::Wrapper, Error as RbError};

mod accounts;

#[crud_table(table_name:bank_accounts)]
#[derive(PartialEq, Clone, Debug)]
pub struct Account {
    server_id: Option<String>,
    user_id: Option<String>,
    pub balance: Option<i32>,
}

impl Account {
    pub fn new(server_id: &u64, user_id: &u64, balance: i32) -> Account {
        Account {
            server_id: Some(server_id.to_string()),
            user_id: Some(user_id.to_string()),
            balance: Some(balance),
        }
    }

    pub fn server_id(&self) -> Result<u64> {
        self.server_id
            .as_ref()
            .ok_or(Error::Schema("server_id field was blank"))
            .and_then(|string| string.parse::<u64>().map_err(Into::into))
    }

    pub fn user_id(&self) -> Result<u64> {
        self.user_id
            .as_ref()
            .ok_or(Error::Schema("user_id field was blank"))
            .and_then(|string| string.parse::<u64>().map_err(Into::into))
    }
}

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

pub struct Db {
    connection: Rbatis,
}

impl Db {
    async fn open(url: &str) -> Result<Db> {
        let rb = Rbatis::new();
        rb.link(url).await.map_err(|err| Error::Rbatis(err))?;

        return Ok(Db { connection: rb });
    }

    async fn all_accounts(&self) -> Result<Vec<Account>> {
        self.connection
            .fetch_list()
            .await
            .map_err(|err| Error::Rbatis(err))
    }

    async fn account(&self, server_id: &u64, user_id: &u64) -> Result<Option<Account>> {
        let query = self
            .connection
            .new_wrapper()
            .eq("server_id", server_id.to_string())
            .eq("user_id", user_id.to_string());

        self.connection
            .fetch_by_wrapper(&query)
            .await
            .map_err(|err| Error::Rbatis(err))
    }

    pub async fn adjust_balance(
        &self,
        server_id: &u64,
        user_id: &u64,
        amount: i64,
    ) -> Result<Account> {
        accounts::adjust_balance(&self.connection, server_id, user_id, amount).await
    }

    pub async fn tip(
        &self,
        server_id: &u64,
        user_id_0: &u64,
        user_id_1: &u64,
    ) -> Result<(Account, Account)> {
        let mut exec = self.connection.acquire_begin().await.map_err(Into::into)?;

        let db = exec.rb;

        let account0: Account = accounts::adjust_balance(db, server_id, user_id_0, 1).await?;
        let account1: Account = accounts::adjust_balance(db, server_id, user_id_1, 2).await?;

        exec.commit().await.map_err(Into::into)?;

        Ok((account0, account1))
    }

    pub async fn untip(
        &self,
        server_id: &u64,
        user_id_0: &u64,
        user_id_1: &u64,
    ) -> Result<(Account, Account)> {
        let mut exec = self.connection.acquire_begin().await.map_err(Into::into)?;

        let db = exec.rb;

        let account0: Account = accounts::adjust_balance(db, server_id, user_id_0, -1).await?;
        let account1: Account = accounts::adjust_balance(db, server_id, user_id_1, -2).await?;

        exec.commit().await.map_err(Into::into)?;

        Ok((account0, account1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    macro_rules! async_run {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    fn open_test_db() -> Db {
        fast_log::init_log("requests.log", 1000, log::Level::Info, None, true);
        //    .expect("unable to start fast_log");
        async_run!(Db::open("sqlite://test.db")).expect("couldn't open test db")
        //Db::open("sqlite://test.db").await.expect("couldn't open test db")
    }

    fn test_account_adjust(db: &Db, account: &Account, amount: i64) {
        let server_id: u64 = account.server_id().expect("unable to parse server_id");
        let user_id: u64 = account.user_id().expect("unable to parse user_id");

        let result_account = async_run!(db.adjust_balance(&server_id, &user_id, amount))
            .expect("unable to raise balance by 1");
        let result_balance: i64 = result_account.balance.unwrap().try_into().unwrap();
        let old_balance: i64 = account.balance.unwrap().try_into().unwrap();
        let expected_balance = old_balance + amount;

        assert_eq!(expected_balance, result_balance);

        let result_account = async_run!(db.adjust_balance(&server_id, &user_id, -amount))
            .expect("unable to raise balance by 1");
        let result_balance: i64 = result_account.balance.unwrap().try_into().unwrap();
        let expected_balance = old_balance;

        assert_eq!(expected_balance, result_balance);
    }

    #[test]
    fn adjustment_tests() {
        let db = open_test_db();
        let accounts = async_run!(db.all_accounts()).expect("unable to load accounts");

        accounts.iter().for_each(|account| {
            log::info!(
                "entry: {:?} {:?} {:?}",
                account.server_id,
                account.user_id,
                account.balance
            );

            let server_id: u64 = account.server_id().expect("unable to parse server_id");
            let user_id: u64 = account.user_id().expect("unable to parse user_id");

            let other_account = async_run!(db.account(&server_id, &user_id))
                .expect("account not found in DB")
                .expect("unable to load other account");
            assert_eq!(account, &other_account);

            test_account_adjust(&db, &account, 5);
            test_account_adjust(&db, &account, 10);
            test_account_adjust(&db, &account, 100);
            test_account_adjust(&db, &account, 87);
        });

        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn tip_test() {
        let db = open_test_db();
        let accounts = async_run!(db.all_accounts()).expect("unable to load accounts");

        let account0 = &accounts[0];
        let account1 = &accounts[1];

        async_run!(db.tip(
            &account0.server_id().unwrap(),
            &account0.user_id().unwrap(),
            &account1.user_id().unwrap(),
        ))
        .expect("unable to perform tip");

        let accounts = async_run!(db.all_accounts()).expect("unable to load accounts");

        let account0_result = &accounts[0];
        let account1_result = &accounts[1];

        let expected_balance0 = account0.balance.unwrap() + 1;
        let expected_balance1 = account1.balance.unwrap() + 2;

        assert_eq!(expected_balance0, account0_result.balance.unwrap());
        assert_eq!(expected_balance1, account1_result.balance.unwrap());

        async_run!(db.untip(
            &account0.server_id().unwrap(),
            &account0.user_id().unwrap(),
            &account1.user_id().unwrap(),
        ))
        .expect("unable to perform untip");

        let accounts = async_run!(db.all_accounts()).expect("unable to load accounts");

        let account0_result = &accounts[0];
        let account1_result = &accounts[1];

        assert_eq!(account0.balance.unwrap(), account0_result.balance.unwrap());
        assert_eq!(account1.balance.unwrap(), account1_result.balance.unwrap());
    }
}

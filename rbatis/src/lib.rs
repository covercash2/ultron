#[macro_use]
extern crate rbatis;

use rbatis::{crud::CRUD, rbatis::Rbatis};

mod accounts;
mod error;

use accounts::Account;
use error::{Error, Result};

pub struct Db {
    connection: Rbatis,
}

impl Db {
    pub async fn open(url: &str) -> Result<Db> {
        let rb = Rbatis::new();
        rb.link(url).await.map_err(|err| Error::Rbatis(err))?;

        return Ok(Db { connection: rb });
    }

    pub async fn all_accounts(&self) -> Result<Vec<Account>> {
        self.connection
            .fetch_list()
            .await
            .map_err(|err| Error::Rbatis(err))
    }

    pub async fn account(&self, server_id: &u64, user_id: &u64) -> Result<Option<Account>> {
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

    pub async fn transfer_coins(
        &self,
        server_id: &u64,
        user_id_0: &u64,
        user_id_1: &u64,
        amount: i64,
    ) -> Result<(Account, Account)> {
        let mut exec = self.connection.acquire_begin().await.map_err(Into::into)?;
        let db = exec.rb;

        let account0: Account = accounts::adjust_balance(db, server_id, user_id_0, -amount).await?;
        let account1: Account = accounts::adjust_balance(db, server_id, user_id_1, amount).await?;

        exec.commit().await.map_err(Into::into)?;

        Ok((account0, account1))
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

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
                account.server_id(),
                account.user_id(),
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

    #[test]
    fn transfer_test() {
        let db = open_test_db();
        let accounts = async_run!(db.all_accounts()).expect("unable to load accounts");

        let account0 = accounts.first().expect("bank_accounts table was empty");
        let account1 = accounts
            .iter()
            .find(|it| it.server_id().unwrap() == account0.server_id().unwrap())
            .expect("could not find another account in this server");

        let transfer_amount = 10;

        let (account0_result, account1_result) = async_run!(db.transfer_coins(
            &account0.server_id().unwrap(),
            &account0.user_id().unwrap(),
            &account1.user_id().unwrap(),
            transfer_amount,
        ))
        .expect("unable to transfer coins");

        let account0_balance: i64 = account0.balance.expect("account0 balance was null").into();
        let account1_balance: i64 = account1.balance.expect("account1 balance was null").into();

        let expected_balance0 = account0_balance - transfer_amount;
        let expected_balance1 = account1_balance + transfer_amount;

        assert_eq!(expected_balance0, account0_result.balance.unwrap() as i64);
        assert_eq!(expected_balance1, account1_result.balance.unwrap() as i64);
    }
}

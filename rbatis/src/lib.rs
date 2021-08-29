#[macro_use]
extern crate rbatis;

use std::fmt::Debug;

use rbatis::{crud::CRUD, rbatis::Rbatis, Error as RbError};

#[crud_table(table_name:bank_accounts)]
#[derive(PartialEq, Clone, Debug)]
pub struct Account {
    pub server_id: Option<String>,
    pub user_id: Option<String>,
    pub balance: Option<i32>,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Rbatis(RbError),
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

    async fn account(&self, server_id: &u64, user_id: &u64) -> Result<Account> {
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
        fast_log::init_log("requests.log", 1000, log::Level::Info, None, true)
            .expect("unable to start fast_log");
        async_run!(Db::open("sqlite://test.db")).expect("couldn't open test db")
        //Db::open("sqlite://test.db").await.expect("couldn't open test db")
    }

    #[test]
    fn it_works() {
        let db = open_test_db();
        let accounts = async_run!(db.all_accounts()).expect("unable to load accounts");

        accounts.iter().for_each(|account| {
            log::info!("{:?} {:?}", account.user_id, account.balance);
            let other_account = async_run!(db.account(
                &account.server_id.as_ref().unwrap().parse::<u64>().unwrap(),
                &account.user_id.as_ref().unwrap().parse::<u64>().unwrap(),
            )).expect("unable to load other account");
            assert_eq!(account, &other_account);
        });

        assert_eq!(2 + 2, 4);
    }
}

use std::convert::TryInto;

use rbatis::{crud::CRUD, rbatis::Rbatis};

use crate::{Error, Result};

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


pub async fn account(db: &Rbatis, server_id: &u64, user_id: &u64) -> Result<Option<Account>> {
    let query = db
        .new_wrapper()
        .eq("server_id", server_id.to_string())
        .eq("user_id", user_id.to_string());

    db.fetch_by_wrapper(&query)
        .await
        .map_err(|err| Error::Rbatis(err))
}

pub async fn adjust_balance(
    db: &Rbatis,
    server_id: &u64,
    user_id: &u64,
    amount: i64,
) -> Result<Account> {
    let amount: i32 = amount
        .try_into()
        .map_err(|_e| Error::CoinOverflow(amount))?;

    let account: Option<Account> = account(db, server_id, user_id).await?;

    if let Some(account) = account {
        let new_balance = account
            .balance
            .unwrap_or(0)
            .checked_add(amount)
            .ok_or(Error::CoinOverflow(amount as i64))?;
        let new_account = Account {
            balance: Some(new_balance),
            ..account
        };

        let query = db
            .new_wrapper()
            .eq("server_id", &new_account.server_id)
            .eq("user_id", &new_account.user_id);

        db.update_by_wrapper(&new_account, &query, &[])
            .await
            .map_err(|err| Error::Rbatis(err))?;

        Ok(new_account)
    } else {
        let new_account = Account::new(server_id, user_id, amount);
        db.save(&new_account, &[])
            .await
            .map_err(|err| Error::Rbatis(err))?;
        Ok(new_account)
    }
}

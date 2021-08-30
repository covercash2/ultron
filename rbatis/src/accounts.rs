use std::convert::TryInto;

use rbatis::{crud::CRUD, rbatis::Rbatis};

use crate::{Account, Error, Result};

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

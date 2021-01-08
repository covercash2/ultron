use std::convert::TryInto;

use diesel::{prelude::*, Connection};

use crate::{error::{Error, Result}, model::BankAccount};

use crate::schema::{bank_accounts::dsl::*};

use crate::Backend;

#[derive(Debug)]
pub struct TransferResult {
    pub to_account: BankAccount,
    pub from_account: BankAccount,
}

pub fn transfer_coins<C: Connection<Backend = Backend>>(
    connection: &C,
    server: &u64,
    from_user: &u64,
    to_user: &u64,
    amount: &i64,
) -> Result<TransferResult> {
    let to_amount: i32 = (*amount).try_into().map_err(|_e| Error::CoinOverflow)?;
    let from_amount: i32 = -to_amount;

    let from_account = BankAccount::new(server, from_user, &from_amount);
    let to_account = BankAccount::new(server, to_user, &to_amount);

    // TODO check if user has enough coins
    connection.transaction::<_, Error, _>(|| {
	let mut record_num = diesel::insert_into(bank_accounts)
	    .values(&from_account)
	    .on_conflict((server_id, user_id))
	    .do_update()
	    .set(balance.eq(balance + from_amount))
	    .execute(connection)?;

	record_num += diesel::insert_into(bank_accounts)
	    .values(&to_account)
	    .on_conflict((server_id, user_id))
	    .do_update()
	    .set(balance.eq(balance + to_amount))
	    .execute(connection)?;

	let to_account: BankAccount = bank_accounts.find((&server.to_string(), &to_user.to_string())).first(connection)?;
	let from_account: BankAccount = bank_accounts.find((&server.to_string(), &from_user.to_string())).first(connection)?;

	if record_num == 2 {
	    Ok(TransferResult {
		to_account, from_account
	    })
	} else {
	    Err(Error::Unexpected(format!("wrong number of changed records: {}", record_num)))
	}
    })
}

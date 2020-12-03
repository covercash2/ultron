use diesel::{Insertable, Queryable};

use super::schema::{bank_accounts, channel_users};

use crate::error::Result;

#[derive(Insertable, Queryable)]
pub struct ChannelUser {
    pub server_id: String,
    pub channel_id: String,
    pub user_id: String,
}

#[derive(Insertable, Queryable)]
pub struct BankAccount {
    server_id: String,
    user_id: String,
    pub balance: i32,
}

impl BankAccount {
    pub fn user_id(&self) -> Result<u64> {
	self.user_id.parse()
	    .map_err(Into::into)
    }
}

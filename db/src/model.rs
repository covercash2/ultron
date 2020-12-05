use diesel::{Insertable, Queryable};

use super::schema::{bank_accounts, channel_users};

use crate::error::Result;

#[derive(Insertable, Queryable)]
pub struct ChannelUser {
    server_id: String,
    channel_id: String,
    user_id: String,
}

impl ChannelUser {
    pub fn new(server: &u64, channel: &u64, user: &u64) -> Self {
        let server_id = server.to_string();
        let channel_id = channel.to_string();
        let user_id = user.to_string();

        ChannelUser {
            server_id,
            channel_id,
            user_id,
        }
    }

    pub fn server_id(&self) -> Result<u64> {
        self.server_id.parse().map_err(Into::into)
    }


    pub fn channel_id(&self) -> Result<u64> {
        self.channel_id.parse().map_err(Into::into)
    }

    pub fn user_id(&self) -> Result<u64> {
        self.user_id.parse().map_err(Into::into)
    }
}

#[derive(Debug, Clone, Insertable, Queryable)]
pub struct BankAccount {
    server_id: String,
    user_id: String,
    pub balance: i32,
}

impl BankAccount {
    pub fn new(server: &u64, user: &u64, balance: &i32) -> BankAccount {
        let server_id = server.to_string();
        let user_id = user.to_string();
        let balance = *balance;
        BankAccount {
            server_id,
            user_id,
            balance,
        }
    }

    pub fn user_id(&self) -> Result<u64> {
        self.user_id.parse().map_err(Into::into)
    }

    pub fn server_id(&self) -> Result<u64> {
        self.server_id.parse().map_err(Into::into)
    }
}

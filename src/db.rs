//! database for ultrons memories
use std::env;

use log::*;

use diesel::{prelude::*, sqlite::SqliteConnection, Queryable};
use dotenv::dotenv;

use crate::schema::channel_users::dsl::*;
use crate::error::Result;

#[derive(Queryable)]
struct ChannelUser {
    server_id: u64,
    user_id: u64,
    balance: i64,
}

pub fn establish_connection() -> Result<SqliteConnection> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")?;
    SqliteConnection::establish(&database_url)
        .map_err(Into::into)
}

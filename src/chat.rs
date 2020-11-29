//! implementation agnostic chat functions
use log::*;

use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct User {
    pub id: u64,
}
#[derive(Debug)]
pub struct Channel {
    pub id: u64,
}
#[derive(Debug)]
pub struct Server {
    pub id: u64,
}

#[derive(Debug)]
pub struct Message {
    pub content: String,
    pub user: User,
    pub channel: Channel,
    pub server: Server,
    pub timestamp: DateTime<Utc>,
}

impl From<u64> for User {
    fn from(id: u64) -> Self {
        User{ id }
    }
}

impl From<u64> for Channel {
    fn from(id: u64) -> Self {
        Channel{ id }
    }
}

impl From<u64> for Server {
    fn from(id: u64) -> Self {
        Server{ id }
    }
}

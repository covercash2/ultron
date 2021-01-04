//! implementation agnostic chat functions
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::data::{ChannelId, ServerId, UserId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
}

#[derive(Debug)]
pub struct Channel {
    pub id: ChannelId,
}

#[derive(Debug)]
pub struct Server {
    pub id: ServerId,
}

#[derive(Debug)]
pub struct Message {
    pub id: u64,
    pub content: String,
    pub user: User,
    pub channel: Channel,
    pub server: Server,
    pub timestamp: DateTime<Utc>,
    pub mentions: Vec<User>,
}

impl From<u64> for User {
    fn from(id: u64) -> Self {
        User { id }
    }
}

impl From<u64> for Channel {
    fn from(id: u64) -> Self {
        Channel { id }
    }
}

impl From<u64> for Server {
    fn from(id: u64) -> Self {
        Server { id }
    }
}

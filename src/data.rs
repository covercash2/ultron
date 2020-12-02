use log::*;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use tokio::{io::AsyncReadExt, fs::OpenOptions};

use crate::error::Result;

pub type UserId = u64;
pub type ChannelId = u64;
pub type ServerId = u64;

const USER_LOG_FILE: &str = "users.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLog {
    map: HashMap<ServerId, HashMap<ChannelId, Vec<UserId>>>,
}

impl UserLog {
    /// Add `user_id` to the channel's daily log.
    pub async fn log_user(
        &mut self,
        server_id: &ServerId,
        channel_id: &ChannelId,
        user: &UserId,
    ) -> Result<()> {
        let channel_log = self.get_or_create_channel_log(server_id, channel_id);
        if channel_log.contains(user) {
            debug!("already logged user: {:?}", user);
            Ok(())
        } else {
            debug!("logging new user: {:?}", user);
            channel_log.push(*user);
            self.save().await
        }
    }

    fn get_or_create_channel_log(
        &mut self,
        server_id: &ServerId,
        channel_id: &ChannelId,
    ) -> &mut Vec<UserId> {
        let channel_map = if self.map.contains_key(server_id) {
            self.map
                .get_mut(server_id)
                .expect("weird error retrieving server user log")
        } else {
            info!("creating user log for server: {}", server_id);
            self.map.insert(*server_id, Default::default());
            self.map
                .get_mut(server_id)
                .expect("unable to get server user log that was just created")
        };
        if channel_map.contains_key(channel_id) {
            channel_map
                .get_mut(channel_id)
                .expect("weird error retrieving channel user log")
        } else {
            info!("creating user log for channel: {}", channel_id);
            channel_map.insert(*channel_id, Default::default());
            channel_map
                .get_mut(channel_id)
                .expect("unable to get channel user log that was just created")
        }
    }

    /// Load saved daily logs
    pub async fn load() -> Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(USER_LOG_FILE)
            .await?;
        let mut content_string = String::new();
        file.read_to_string(&mut content_string).await?;
        if content_string.is_empty() {
            let map = Default::default();
            Ok(UserLog { map })
        } else {
            serde_json::from_str(&content_string).map_err(Into::into)
        }
    }

    /// Save daily user log
    pub async fn save(&self) -> Result<()> {
        let json: String = serde_json::to_string(self)?;
        tokio::fs::write(USER_LOG_FILE, json)
            .await
            .map_err(Into::into)
    }
}

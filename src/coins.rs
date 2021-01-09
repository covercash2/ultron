//! A simple economy database.
use std::{collections::HashMap, sync::Arc};

use chrono::Duration;
use log::*;
use tokio::{fs::OpenOptions, prelude::*, sync::Mutex};

use serde::{Deserialize, Serialize};
use serde_json;

use chrono::{DateTime, Utc};

use db::{model::BankAccount, Db, TransferResult};

use crate::data::{Database, ServerId, UserId};
use crate::error::{Error, Result};

/// the id of the member card in the database
const ITEM_ID_MEMBER_CARD: u64 = 1;
/// The log file for the daily logins
const DAILY_LOG_FILE: &str = "daily_log.json";
/// The amount of coins to give to each user that asks once per day
const DAILY_AMOUNT: i64 = 10;

/// Get the next epoch for the daily allowances.
pub fn daily_epoch() -> DateTime<Utc> {
    let epoch = Utc::today().and_hms(0, 0, 0);
    if epoch < Utc::now() {
        epoch + Duration::days(1)
    } else {
        epoch
    }
}

pub async fn all_balances(
    db: &Database,
    server_id: u64,
    channel_id: u64,
) -> Result<Vec<BankAccount>> {
    db.transaction(|db| {
        db.channel_user_balances(&server_id, &channel_id)
            .map_err(Into::into)
    })
    .await
}

pub async fn user_account(db: &Database, server_id: u64, user_id: u64) -> Result<BankAccount> {
    db.transaction(|db| db.user_account(&server_id, &user_id).map_err(Into::into))
        .await
}

/// returns Ok(true) if the daily was successful.
/// returns Ok(false) if the user has already received a daily
pub async fn add_daily(
    db: &Database,
    daily_log: &Arc<Mutex<DailyLog>>,
    server_id: u64,
    user_id: u64,
) -> Result<bool> {
    let gets_daily = {
        let mut daily_log = daily_log.lock().await;
        let gets_daily = daily_log.log_user(&server_id, user_id);
        daily_log.save().await?;
        gets_daily
    };

    if gets_daily {
        db.transaction(|db: &Db| {
            let is_member = db.user_has_item(server_id, user_id, ITEM_ID_MEMBER_CARD)?;
            let amount = if is_member {
                DAILY_AMOUNT * 2
            } else {
                DAILY_AMOUNT
            };
            let num_records = db.increment_balance(&server_id, &user_id, &amount)?;
            if num_records == 0 {
                Err(Error::Unknown("no records changed".to_owned()))
            } else {
                Ok(true)
            }
        })
        .await
    } else {
        Ok(false)
    }
}

/// transfer coins from one user to another
pub async fn transfer(
    db: &Database,
    server_id: u64,
    from_user_id: u64,
    to_user_id: u64,
    amount: i64,
) -> Result<TransferResult> {
    db.transaction(|db| {
        db.transfer_coins(&server_id, &from_user_id, &to_user_id, &amount)
            .map_err(Into::into)
    })
    .await
}

/// give coins and receive a coin for participating
pub async fn tip(db: &Database, server_id: u64, from_user_id: u64, to_user_id: u64) -> Result<()> {
    db.transaction(|db| {
        db.tip(&server_id, &from_user_id, &to_user_id)
            .map_err(Into::into)
    })
    .await
}

/// undo tip action
pub async fn untip(
    db: &Database,
    server_id: u64,
    from_user_id: u64,
    to_user_id: u64,
) -> Result<()> {
    db.transaction(|db| {
        db.untip(&server_id, &from_user_id, &to_user_id)
            .map_err(Into::into)
    })
    .await
}

/// A log to keep track of who's logged in today and the next epoch when the daily
/// time resets.
#[derive(Serialize, Deserialize, Debug)]
pub struct DailyLog {
    /// The next time the logs will reset.
    epoch: DateTime<Utc>,
    /// A map of channels and its users who have logged in.
    /// As users log in to each channel, they are added to `server_id->Vec<UserId>`
    map: HashMap<ServerId, Vec<UserId>>,
}

impl DailyLog {
    /// Add `user_id` to the channel's daily log.
    /// Return true if the user has not yet logged in today.
    fn log_user(&mut self, server_id: &ServerId, user_id: UserId) -> bool {
        // increment epoch if necessary
        if Utc::now() > self.epoch {
            debug!("epoch has passed: {:?}", self.epoch);
            self.epoch = daily_epoch();
            self.clear();
            debug!("new epoch: {:?}", self.epoch);
        }

        let channel_log = self.get_or_create(server_id);
        if channel_log.contains(&user_id) {
            // bad user
            false
        } else {
            // add to log
            channel_log.push(user_id);
            true
        }
    }

    /// Get a channel's user log or create it if it doesn't exist
    fn get_or_create(&mut self, server_id: &ServerId) -> &mut Vec<UserId> {
        if self.map.contains_key(server_id) {
            return self
                .map
                .get_mut(server_id)
                .expect("weird error retrieving daily log");
        }
        info!("creating daily log for channel: {}", server_id);
        self.map.insert(*server_id, Vec::new());
        self.map
            .get_mut(server_id)
            .expect("unable to get daily log that was just created")
    }

    /// Clear the user logs
    fn clear(&mut self) {
        self.map.clear();
    }

    /// Load saved daily logs
    pub async fn load() -> Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(DAILY_LOG_FILE)
            .await?;
        let mut content_string = String::new();
        file.read_to_string(&mut content_string).await?;
        if content_string.is_empty() {
            let map = HashMap::new();
            let epoch = daily_epoch();
            Ok(DailyLog { map, epoch })
        } else {
            serde_json::from_str(&content_string).map_err(Into::into)
        }
    }

    /// Save daily user log
    pub async fn save(&self) -> Result<()> {
        let json: String = serde_json::to_string(self)?;
        tokio::fs::write(DAILY_LOG_FILE, json)
            .await
            .map_err(Into::into)
    }
}

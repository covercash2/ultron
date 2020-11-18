use log::*;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json;

use tokio::{io::AsyncReadExt, fs::OpenOptions};

use crate::error::Result;

const DATA_FILE: &str = "accounts.json";

type ChannelId = u64;

#[derive(Serialize, Deserialize, Debug)]
pub struct Accounts {
    ledgers: HashMap<u64, Ledger>,
}

impl Accounts {
    /// Load saved account data
    pub async fn load() -> Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(DATA_FILE)
            .await?;
        let mut content_string = String::new();
        file.read_to_string(&mut content_string).await?;
        if content_string.is_empty() {
            let ledgers = HashMap::new();
            Ok(Accounts { ledgers })
        } else {
            serde_json::from_str(&content_string).map_err(Into::into)
        }
    }

    /// Save account data
    pub async fn save(&self) -> Result<()> {
        let json: String = serde_json::to_string(self)?;
        tokio::fs::write(DATA_FILE, json).await.map_err(Into::into)
    }

    pub fn get_or_create(&mut self, channel_id: &ChannelId) -> &Ledger {
        if self.ledgers.contains_key(channel_id) {
            return self
                .ledgers
                .get(channel_id)
                .expect("weird error retrieving ledger");
        }
        info!("creating accounts for channel: {:?}", channel_id);
        self.ledgers.insert(*channel_id, Ledger::default());
        self.ledgers
            .get(channel_id)
            .expect("unable to get the ledger that was just created")
    }

    pub fn get_or_create_mut(&mut self, channel_id: &ChannelId) -> &mut Ledger {
        if self.ledgers.contains_key(channel_id) {
            return self
                .ledgers
                .get_mut(channel_id)
                .expect("weird error retrieving ledger");
        }
        info!("creating accounts for channel: {:?}", channel_id);
        self.ledgers.insert(*channel_id, Ledger::default());
        self.ledgers
            .get_mut(channel_id)
            .expect("unable to get the ledger that was just created")
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ledger {
    map: HashMap<u64, i64>,
}

impl Ledger {
    /// Increment the user account by `amount`.
    /// This function can be used to decrement the account by passing a negative number.
    pub fn increment_balance(&mut self, user: &u64, amount: i64) {
        match self.map.get_mut(user) {
            // TODO maybe check for overflows here
            Some(balance) => *balance += amount,
            None => {
                if let Some(old_amount) = self.map.insert(*user, amount) {
                    error!("value overwritten for user: {} -- {}", user, old_amount);
                }
            }
        }
    }

    pub fn transfer(&mut self, from_user: &u64, to_user: &u64, amount: i64) {
        self.increment_balance(&from_user, -amount);
        self.increment_balance(&to_user, amount);
    }

    /// Get the balance of the user account or create it and initialize it with 0
    pub fn get_balance(&mut self, user: &u64) -> i64 {
        match self.map.get(user) {
            Some(&amount) => amount,
            None => {
                self.map.insert(*user, 0);
                0
            }
        }
    }

    pub fn get_balances(&mut self, users: Vec<u64>) -> Vec<(u64, i64)> {
        users
            .iter()
            .map(|user| (*user, self.get_balance(user)))
            .collect()
    }

    pub fn get_all_balances(&self) -> Vec<(u64, i64)> {
        self.map
            .iter()
            .map(|(uid, amount)| (*uid, *amount))
            .collect()
    }
}

impl Default for Ledger {
    fn default() -> Self {
	return Ledger {
	    map: HashMap::new(),
	}
    }
}

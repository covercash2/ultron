//! A simple economy database.
use std::collections::HashMap;

use chrono::Duration;
use log::*;
use tokio::{
    fs::OpenOptions,
    prelude::*,
    sync::mpsc::{Receiver, Sender},
};

use serde::{Deserialize, Serialize};
use serde_json;

use chrono::{DateTime, Utc};

use crate::error::Result;

mod ledger;

use ledger::Accounts;

/// The log file for the daily logins
const DAILY_LOG_FILE: &str = "daily_log.json";
/// The amount of coins to give to each user that asks once per day
const DAILY_AMOUNT: i64 = 10;

type ChannelId = u64;
type UserId = u64;
type Account = (UserId, i64);

/// Get the next epoch for the daily allowances.
fn daily_epoch() -> DateTime<Utc> {
    let epoch = Utc::today().and_hms(0, 0, 0);
    if epoch < Utc::now() {
        epoch + Duration::days(1)
    } else {
        epoch
    }
}

/// Interactions with the Bank are handled through transactions.
/// These transactions are sent over channels in the [`bank_loop`]
/// to be processed by [`Bank::process_transaction`].
#[derive(Debug)]
pub enum Transaction {
    /// Transfer coins from one user to another
    Transfer {
        channel_id: ChannelId,
        from_user: UserId,
        to_user: UserId,
        amount: i64,
    },
    /// Dump the account data
    GetAllBalances(ChannelId),
    Tip {
        channel_id: ChannelId,
        from_user: UserId,
        to_user: UserId,
    },
    /// Give some coins to a user once per day
    Daily {
        channel_id: ChannelId,
        user_id: UserId,
        timestamp: DateTime<Utc>,
    },
}

/// This type is returned from [`Bank::process_transaction`].
/// It uses a `Vec` of tuples to represent user ids and the associated account balance after a transaction
/// completes.
#[derive(Debug)]
pub struct Receipt {
    pub transaction: Transaction,
    pub account_results: Vec<Account>,
    pub status: TransactionStatus,
}

#[derive(Debug)]
pub enum TransactionStatus {
    Complete,
    BadDailyRequest { next_epoch: DateTime<Utc> },
    SelfTip,
}

impl Receipt {
    pub fn iter(&self) -> impl Iterator<Item = &Account> {
        self.account_results.iter()
    }
}

/// This function runs a loop that waits for transactions to come in on the
/// `transaction_receiver` (see: [`tokio::sync::mpsc`]).
/// If some `Receipt` value is returned from the transaction, it is sent across the
/// `output_sender`.
/// This loop runs until the `transaction_receiver`'s [`Sender`] sides are closed.
pub async fn bank_loop(
    mut bank: Bank,
    mut transaction_receiver: Receiver<Transaction>,
    mut output_sender: Sender<Receipt>,
) {
    debug!("bank loop started");
    while let Some(transaction) = transaction_receiver.recv().await {
        debug!("transaction received: {:?}", transaction);

        let receipt = bank.process_transaction(transaction).await;

        if let Err(err) = output_sender.send(receipt).await {
            error!("error sending receipt: {:?}", err);
        }
    }
    debug!("bank loop finished");
}

/// The main structure for storing account information.
#[derive(Debug)]
pub struct Bank {
    ledgers: Accounts,
    /// A map to keep track of which users have logged in today
    daily_log: DailyLog,
}

impl Bank {
    /// Process a transaction and return a [`Receipt`]
    pub async fn process_transaction(&mut self, transaction: Transaction) -> Receipt {
        match transaction {
            Transaction::Transfer {
                channel_id,
                from_user,
                to_user,
                amount,
            } => {
                let ledger = self.ledgers.get_or_create_mut(&channel_id);
                ledger.transfer(&from_user, &to_user, amount);
                let account_results = ledger.get_balances(vec![from_user, to_user]);

                if let Err(err) = self.save().await {
                    error!("unable to save ledger: {:?}", err);
                }

                Receipt {
                    transaction,
                    account_results,
                    status: TransactionStatus::Complete,
                }
            }
            Transaction::GetAllBalances(channel_id) => {
                let ledger = self.ledgers.get_or_create(&channel_id);
                let account_results = ledger.get_all_balances();

                Receipt {
                    transaction,
                    account_results,
                    status: TransactionStatus::Complete,
                }
            }
            Transaction::Tip {
                channel_id,
                from_user,
                to_user,
            } => {
                if from_user == to_user {
                    let account_results = Vec::new();
                    Receipt {
                        transaction,
                        account_results,
                        status: TransactionStatus::SelfTip,
                    }
                } else {
                    let ledger = self.ledgers.get_or_create_mut(&channel_id);
                    ledger.increment_balance(&to_user, 2);
                    ledger.increment_balance(&from_user, 1);
                    let account_results = ledger.get_balances(vec![from_user, to_user]);

                    if let Err(err) = self.ledgers.save().await {
                        error!("unable to save ledger: {:?}", err);
                    }

                    Receipt {
                        transaction,
                        account_results,
                        status: TransactionStatus::Complete,
                    }
                }
            }
            Transaction::Daily {
                channel_id,
                user_id,
                timestamp,
            } => {
                info!("unhandled timestamp: {:?}", timestamp);

                if self.daily_log.log_user(&channel_id, user_id) {
                    // first log today
                    // award daily
                    debug!("awarding daily to user{} on channel{}", user_id, channel_id);
                    let ledger = self.ledgers.get_or_create_mut(&channel_id);
                    ledger.increment_balance(&user_id, DAILY_AMOUNT);

                    let account_results = ledger.get_balances(vec![user_id]);
                    let status = TransactionStatus::Complete;

                    if let Err(err) = self.daily_log.save().await {
                        error!("error saving daily log: {:?}", err);
                    }

                    Receipt {
                        transaction,
                        account_results,
                        status,
                    }
                } else {
                    // user has already logged today
                    // return bad user message
                    debug!(
                        "rejecting daily request from user{} on channel{}",
                        user_id, channel_id
                    );
                    let account_results = vec![];
                    let status = TransactionStatus::BadDailyRequest {
			next_epoch: self.daily_log.epoch.clone()
		    };

                    Receipt {
                        transaction,
                        account_results,
                        status,
                    }
                }
            }
        }
    }

    /// Load saved account data
    pub async fn load() -> Result<Self> {
        let ledgers = Accounts::load().await?;
        let daily_log = DailyLog::load().await?;

        Ok(Bank { ledgers, daily_log })
    }

    /// Save account data
    pub async fn save(&self) -> Result<()> {
        self.ledgers.save().await?;
        self.daily_log.save().await
    }
}

/// A log to keep track of who's logged in today and the next epoch when the daily
/// time resets.
#[derive(Serialize, Deserialize, Debug)]
pub struct DailyLog {
    /// The next time the logs will reset.
    epoch: DateTime<Utc>,
    /// A map of channels and its users who have logged in.
    /// As users log in to each channel, they are added to `channel_id->Vec<UserId>`
    map: HashMap<ChannelId, Vec<UserId>>,
}

impl DailyLog {
    /// Add `user_id` to the channel's daily log.
    /// Return true if the user has not yet logged in today.
    fn log_user(&mut self, channel_id: &ChannelId, user_id: UserId) -> bool {
        // increment epoch if necessary
        if Utc::now() > self.epoch {
            debug!("epoch has passed: {:?}", self.epoch);
            self.epoch = daily_epoch();
            self.clear();
            debug!("new epoch: {:?}", self.epoch);
        }

        let channel_log = self.get_or_create(channel_id);
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
    fn get_or_create(&mut self, channel_id: &ChannelId) -> &mut Vec<UserId> {
        if self.map.contains_key(channel_id) {
            return self
                .map
                .get_mut(channel_id)
                .expect("weird error retrieving daily log");
        }
        info!("creating daily log for channel: {}", channel_id);
        self.map.insert(*channel_id, Vec::new());
        self.map
            .get_mut(channel_id)
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

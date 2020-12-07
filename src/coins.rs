//! A simple economy database.
use std::{collections::HashMap, sync::Arc};

use chrono::Duration;
use log::*;
use tokio::{
    fs::OpenOptions,
    prelude::*,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};

use serde::{Deserialize, Serialize};
use serde_json;

use chrono::{DateTime, Utc};

use db::Db;

use crate::data::{ChannelId, ServerId, UserId};
use crate::error::Result;

mod transaction;

pub use transaction::{Operation, Transaction, TransactionSender, TransactionStatus};

/// The log file for the daily logins
const DAILY_LOG_FILE: &str = "daily_log.json";
/// The amount of coins to give to each user that asks once per day
const DAILY_AMOUNT: i64 = 10;

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

/// This type is returned from [`Bank::process_transaction`].
/// It uses a `Vec` of tuples to represent user ids and the associated account balance after a transaction
/// completes.
#[derive(Debug, Clone)]
pub struct Receipt {
    pub transaction: Transaction,
    pub account_results: Vec<Account>,
    pub status: TransactionStatus,
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

        match bank.process_transaction(transaction).await {
            Ok(receipt) => {
                if let Err(err) = output_sender.send(receipt).await {
                    error!("error sending receipt: {:?}", err);
                }
            }
            Err(err) => error!("error processing transaction: {:?}", err),
        }
    }
    debug!("bank loop finished");
}

/// The main structure for storing account information.
#[derive(Debug)]
pub struct Bank {
    db: Arc<Mutex<Db>>,
    /// A map to keep track of which users have logged in today
    daily_log: DailyLog,
}

impl Bank {
    /// Process a transaction and return a [`Receipt`]
    pub async fn process_transaction(&mut self, transaction: Transaction) -> Result<Receipt> {
        let server_id = transaction.server_id;
        let channel_id = transaction.channel_id;
        let from_user_id = transaction.from_user;

        self.log_user(&server_id, &transaction.channel_id, &transaction.from_user)
            .await?;

        let receipt = match transaction.operation {
            Operation::Transfer { to_user, amount } => {
                self.transfer_coins(&server_id, &from_user_id, &to_user, amount)
                    .await?;

                let account_results = self
                    .get_balances(&server_id, vec![from_user_id, to_user])
                    .await?;

                if let Err(err) = self.save().await {
                    error!("unable to save ledger: {:?}", err);
                }

                Receipt {
                    transaction,
                    account_results,
                    status: TransactionStatus::Complete,
                }
            }
            Operation::GetAllBalances => {
                let account_results = self.get_all_balances(&server_id, &channel_id).await?;

                Receipt {
                    transaction,
                    account_results,
                    status: TransactionStatus::Complete,
                }
            }
            Operation::Tip { to_user } => {
                if from_user_id == to_user {
                    let account_results = Vec::new();
                    Receipt {
                        transaction,
                        account_results,
                        status: TransactionStatus::SelfTip,
                    }
                } else {
                    self.tip(&server_id, &from_user_id, &to_user).await?;

                    let account_results = self
                        .get_balances(&server_id, vec![from_user_id, to_user])
                        .await?;

                    Receipt {
                        transaction,
                        account_results,
                        status: TransactionStatus::Complete,
                    }
                }
            }
            Operation::Untip { to_user } => {
                if from_user_id == to_user {
                    let account_results = Vec::new();
                    Receipt {
                        transaction,
                        account_results,
                        status: TransactionStatus::SelfTip, // TODO new error type?
                    }
                } else {
                    self.untip(&server_id, &from_user_id, &to_user).await?;
                    let account_results = self
                        .get_balances(&server_id, vec![from_user_id, to_user])
                        .await?;

                    Receipt {
                        transaction,
                        account_results,
                        status: TransactionStatus::Complete,
                    }
                }
            }
            Operation::Daily { timestamp } => {
                let user_id = from_user_id;
                info!("unhandled timestamp: {:?}", timestamp);
                if self.daily_log.log_user(&server_id, user_id) {
                    // first log today
                    // award daily
                    debug!("awarding daily to user{} on channel{}", user_id, server_id);

                    let status = if let Err(err) = self.add_daily(&server_id, &user_id).await {
                        error!("unable to add daily to account: {:?}", err);
                        TransactionStatus::DbError
                    } else {
                        TransactionStatus::Complete
                    };
                    let account_results = self.get_balances(&server_id, vec![user_id]).await?;

                    // TODO db daily_log?
                    if let Err(err) = self.daily_log.save().await {
                        error!("error saving daily log: {:?}", err);
                    }

		    // save daily log
		    self.save().await?;

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
                        user_id, server_id
                    );
                    let account_results = vec![];
                    let status = TransactionStatus::BadDailyRequest {
                        next_epoch: self.daily_log.epoch.clone(),
                    };

		    // save daily log
		    self.save().await?;

                    Receipt {
                        transaction,
                        account_results,
                        status,
                    }
                }
            }
            Operation::GetUserBalance => {
                let user_id = from_user_id;
                let account_results = self.get_balances(&server_id, vec![user_id]).await?;
                let status = TransactionStatus::Complete;

                Receipt {
                    transaction,
                    account_results,
                    status,
                }
            }
        };

        Ok(receipt)
    }

    /// Load saved account data
    pub async fn load<S: AsRef<str>>(database_url: S) -> Result<Self> {

	info!("using database: {}", database_url.as_ref());

        let db = Arc::new(Mutex::new(Db::open(database_url.as_ref())?));

        let daily_log = DailyLog::load().await?;

        Ok(Bank {
            daily_log,
            db,
        })
    }

    /// Save account data
    pub async fn save(&self) -> Result<()> {
        self.daily_log.save().await
    }

    /// this function only gets balances that have the same server and channel,
    /// despite the name.
    async fn get_all_balances(
        &mut self,
        server_id: &u64,
        channel_id: &u64,
    ) -> Result<Vec<(u64, i64)>> {
        let db = self.db.lock().await;
        db.channel_user_balances(server_id, channel_id)?
            .into_iter()
            .map(|account| Ok((account.user_id()?, account.balance.into())))
            .collect::<Result<Vec<(u64, i64)>>>()
    }

    async fn get_balances(
        &mut self,
        server_id: &u64,
        user_ids: Vec<u64>,
    ) -> Result<Vec<(u64, i64)>> {
        let db = self.db.lock().await;
        let balances: Vec<(u64, i64)> = db
            .user_accounts(server_id, &user_ids)?
            .iter()
            .map(|account| Ok((account.user_id()?, account.balance.into())))
            .collect::<Result<Vec<(u64, i64)>>>()?;

        Ok(balances)
    }

    async fn transfer_coins(
        &mut self,
        server_id: &u64,
        from_user: &u64,
        to_user: &u64,
        amount: i64,
    ) -> Result<usize> {
        let db = self.db.lock().await;
        let record_num = db.transfer_coins(server_id, from_user, to_user, &amount)?;

        Ok(record_num)
    }

    async fn add_daily(&mut self, server_id: &u64, user_id: &u64) -> Result<usize> {
        let db = self.db.lock().await;
        db.increment_balance(server_id, user_id, &DAILY_AMOUNT)
            .map_err(Into::into)
    }

    async fn tip(&mut self, server_id: &u64, from_user: &u64, to_user: &u64) -> Result<usize> {
        let db = self.db.lock().await;
        db.tip(server_id, from_user, to_user)
            .map_err(Into::into)
    }

    async fn untip(&mut self, server_id: &u64, from_user: &u64, to_user: &u64) -> Result<usize> {
        let db = self.db.lock().await;
        db.untip(server_id, from_user, to_user)
            .map_err(Into::into)
    }

    async fn log_user(&mut self, server_id: &u64, channel_id: &u64, user_id: &u64) -> Result<()> {
        let db = self.db.lock().await;
        let record_num = db.log_user(server_id, channel_id, user_id)?;

        if record_num == 1 {
            debug!(
                "user logged: #s{} #c{} #u{}",
                server_id, channel_id, user_id
            );
        }

        Ok(())
    }
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

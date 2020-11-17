use std::collections::HashMap;

use log::*;
use tokio::{
    fs::OpenOptions,
    prelude::*,
    sync::mpsc::{Receiver, Sender},
};

use serde::{Deserialize, Serialize};
use serde_json;

use crate::error::Result;

mod ledger;

use ledger::Ledger;

const DATA_FILE: &'static str = "accounts.json";

type ChannelId = u64;
type UserId = u64;
type Account = (UserId, i64);

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
}

/// This type is returned from [`Bank::process_transaction`].
/// It uses a `Vec` of tuples to represent user ids and the associated account balance after a transaction
/// completes.
#[derive(Debug)]
pub struct Receipt {
    transaction: Transaction,
    account_results: Vec<Account>,
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
#[derive(Serialize, Deserialize)]
pub struct Bank {
    ledgers: HashMap<ChannelId, Ledger>,
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
                let ledger = self.get_or_create_ledger_mut(&channel_id);
                ledger.transfer(&from_user, &to_user, amount);
                let account_results = ledger.get_balances(vec![from_user, to_user]);

                if let Err(err) = self.save().await {
                    error!("unable to save ledger: {:?}", err);
                }

                Receipt {
                    transaction,
                    account_results,
                }
            }
            Transaction::GetAllBalances(channel_id) => {
                let ledger = self.get_or_create_ledger(&channel_id);
                let account_results = ledger.get_all_balances();

                Receipt {
                    transaction,
                    account_results,
                }
            }
            Transaction::Tip {
                channel_id,
                from_user,
                to_user,
            } => {
                let ledger = self.get_or_create_ledger_mut(&channel_id);
                ledger.increment_balance(&to_user, 2);
                ledger.increment_balance(&from_user, 1);
                let account_results = ledger.get_balances(vec![from_user, to_user]);

                Receipt {
                    transaction,
                    account_results,
                }
            }
        }
    }

    fn get_or_create_ledger(&mut self, channel_id: &ChannelId) -> &Ledger {
        if self.ledgers.contains_key(channel_id) {
            return self
                .ledgers
                .get(channel_id)
                .expect("weird error retrieving ledger");
        }
        info!("creating accounts for channel");
        self.ledgers.insert(*channel_id, Ledger::default());
        self.ledgers
            .get(channel_id)
            .expect("unable to get the ledger that was just created")
    }

    fn get_or_create_ledger_mut(&mut self, channel_id: &ChannelId) -> &mut Ledger {
        if self.ledgers.contains_key(channel_id) {
            return self
                .ledgers
                .get_mut(channel_id)
                .expect("weird error retrieving ledger");
        }
        info!("creating accounts for channel");
        self.ledgers.insert(*channel_id, Ledger::default());
        self.ledgers
            .get_mut(channel_id)
            .expect("unable to get the ledger that was just created")
    }

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
            Ok(Bank { ledgers })
        } else {
            serde_json::from_str(&content_string).map_err(Into::into)
        }
    }

    /// Save account data
    pub async fn save(&self) -> Result<()> {
        let json: String = serde_json::to_string(self)?;
        tokio::fs::write(DATA_FILE, json).await.map_err(Into::into)
    }
}

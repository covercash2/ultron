use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{BufReader, Read},
};

use log::*;
use tokio::sync::mpsc::Receiver;

use serde::{Deserialize, Serialize};
use serde_json;
use tokio::sync::mpsc::Sender;

use crate::error::Result;

mod ledger;

use ledger::Ledger;

/// This type is returned from [`Bank::process_transaction`].
/// It uses a `Vec` of tuples to represent user ids and the associated account balance after a transaction
/// completes.
pub type Receipt = Vec<(u64, i64)>;

type ChannelId = u64;
type UserId = u64;

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
        if let Some(receipt) = bank.process_transaction(&transaction) {
            if let Err(err) = output_sender.send(receipt).await {
                error!("error sending receipt: {:?}", err);
            }
        } else {
            info!("no receipt for transaction: {:?}", transaction);
        }
    }
    debug!("bank loop finished");
}

/// The main structure for storing account information.
#[derive(Serialize, Deserialize)]
pub struct Bank {
    map: HashMap<u64, i64>,
    ledgers: HashMap<ChannelId, Ledger>,
}

impl Default for Bank {
    fn default() -> Self {
        Bank::load("accounts.json").expect("unable to load default accounts file")
    }
}

impl Bank {
    /// Process transactions and return a [`Receipt`] if appropriate.
    // TODO return result
    pub fn process_transaction(&mut self, transaction: &Transaction) -> Option<Receipt> {
        match transaction {
            Transaction::Transfer {
                channel_id,
                from_user,
                to_user,
                amount,
            } => {
                let ledger = self.get_or_create_ledger_mut(&channel_id);
                ledger.transfer(&from_user, &to_user, *amount);
                Some(ledger.get_balances(vec![*from_user, *to_user]))
            }
            Transaction::GetAllBalances(channel_id) => {
                let ledger = self.get_or_create_ledger(channel_id);
                let receipt = ledger.get_all_balances();

                if receipt.is_empty() {
                    warn!("no entries in bank");
                    None
                } else {
                    Some(receipt)
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

    fn transfer(&mut self, from_user: &u64, to_user: &u64, amount: i64) {
        self.increment_balance(&from_user, -amount);
        self.increment_balance(&to_user, amount);
    }

    fn get_balances(&mut self, users: Vec<u64>) -> Receipt {
        users
            .iter()
            .map(|user| (*user, self.get_balance(user)))
            .collect()
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

    /// Load saved account data
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true) // create the file if it doesn't exist
            .open(path)?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;

        // TODO load accounts
        // let map = if contents.is_empty() {
        //     HashMap::new()
        // } else {
        //     serde_json::from_str(&contents)?
        // };

        let map = HashMap::new();
        let ledgers = HashMap::new();

        return Ok(Bank { map, ledgers });
    }

    /// Save account data
    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string(self)?;
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            // TODO put this somewhere better
            .open("accounts.json")?;
        let mut buf_writer = BufWriter::new(file);
        buf_writer.write_all(json.as_bytes())?;

        Ok(())
    }
}

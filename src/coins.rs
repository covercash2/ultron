use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{BufReader, Read},
};

use log::{debug, error};
use tokio::sync::mpsc::Receiver;

use serde::{Deserialize, Serialize};
use serde_json;
use tokio::sync::mpsc::Sender;

use crate::error::Result;

#[derive(Debug)]
pub enum Transaction {
    Transfer {
        from_user: u64,
        to_user: u64,
        amount: i64,
    },
    GetBalance {
        users: Vec<u64>,
    },
}

pub type Receipt = Vec<(u64, i64)>;

pub async fn bank_loop(
    mut bank: Bank,
    mut transaction_receiver: Receiver<Transaction>,
    mut output_sender: Sender<Receipt>,
) {
    debug!("bank loop started");
    while let Some(transaction) = transaction_receiver.recv().await {
        debug!("transaction received: {:?}", transaction);
        if let Some(receipt) = bank.process_transaction(transaction) {
            if let Err(err) = output_sender.send(receipt).await {
                error!("error sending receipt: {:?}", err);
            }
        }
    }
    debug!("bank loop finished");
}

#[derive(Serialize, Deserialize)]
pub struct Bank {
    map: HashMap<u64, i64>,
}

impl Default for Bank {
    fn default() -> Self {
        Bank::load("accounts.json").expect("unable to load default accounts file")
    }
}

impl Bank {
    pub fn process_transaction(&mut self, transaction: Transaction) -> Option<Receipt> {
        match transaction {
            Transaction::Transfer {
                from_user,
                to_user,
                amount,
            } => {
                self.transfer(&from_user, &to_user, amount);
                Some(self.get_balances(vec![from_user, to_user]))
            }
            Transaction::GetBalance { users } => Some(self.get_balances(users)),
        }
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

    /// get the balance of the user account or create it and initialize it with 0
    pub fn get_balance(&mut self, user: &u64) -> i64 {
        match self.map.get(user) {
            Some(&amount) => amount,
            None => {
                self.map.insert(*user, 0);
                0
            }
        }
    }

    /// increment the user account by `amount`.
    /// this function can be used to decrement the account by passing a negative number
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

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true) // create the file if it doesn't exist
            .open(path)?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;

        let map = if contents.is_empty() {
            HashMap::new()
        } else {
            serde_json::from_str(&contents)?
        };

        return Ok(Bank { map });
    }

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

use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufReader, Read},
};

use log::debug;
use tokio::sync::mpsc::Receiver;

use serde::{Deserialize, Serialize};
use serde_json;

use crate::error::Result;

type Transaction = (u64, i64);

pub async fn bank_loop(bank: Bank, mut transaction_receiver: Receiver<Transaction>) {
    debug!("bank loop started");
    while let Some(transaction) = transaction_receiver.recv().await {
	debug!("transaction received: {:?}", transaction);
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

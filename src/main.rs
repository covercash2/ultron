#![feature(async_closure)]
use std::{env, path::PathBuf};

use dotenv::dotenv;
use log::*;

use tokio::sync::mpsc::{channel, Receiver, Sender};

mod logger;

mod chat;
mod coins;
mod commands;
mod data;
mod error;

mod discord;
mod gambling;
mod tokens;

use coins::{bank_loop, Bank, Receipt, Transaction, TransactionSender};
use discord::Handler;
use tokens::load_token;

#[tokio::main]
async fn main() {
    logger::init();

    // load .env file
    dotenv().ok();
    // get sqlite database file path
    let database_url: String = env::var("TEST_DB_URL")
        .or(env::var("PROD_DB_URL"))
        .unwrap_or_else(|err| {
            warn!("no DB_URL set. falling back to test.db: {:?}", err);
            "test.db".to_owned()
        });

    info!(
        "using database: {:?}",
        PathBuf::from(&database_url)
            .canonicalize()
            .expect("unable to get file name")
    );

    let discord_token = load_token(tokens::DISCORD_TOKEN).expect("unable to load discord token");

    let (transaction_sender, transaction_receiver): (Sender<Transaction>, Receiver<Transaction>) =
        channel(100);
    let (receipt_sender, receipt_receiver): (Sender<Receipt>, Receiver<Receipt>) = channel(100);

    let bank_channel = TransactionSender::new(transaction_sender, receipt_receiver);

    let event_handler = Handler::new(bank_channel);

    let bank = Bank::load(database_url)
        .await
        .expect("unable to load bank file");

    let _bank_thread = tokio::task::spawn(bank_loop(bank, transaction_receiver, receipt_sender));

    if let Err(err) = discord::run(event_handler, discord_token).await {
        error!("error running discord client: {:?}", err);
    }
}

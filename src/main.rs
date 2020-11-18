#![feature(async_closure)]
use log::error;
use pretty_env_logger;

use tokio::sync::mpsc::{channel, Receiver, Sender};

mod coins;
mod commands;
mod error;

mod discord;

mod tokens;

use coins::{bank_loop, Bank, Receipt, Transaction};
use discord::Handler;
use tokens::load_token;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let discord_token = load_token(tokens::DISCORD_TOKEN).expect("unable to load discord token");

    let (transaction_sender, transaction_receiver): (Sender<Transaction>, Receiver<Transaction>) =
        channel(100);
    let (receipt_sender, receipt_receiver): (Sender<Receipt>, Receiver<Receipt>) = channel(100);

    let event_handler = Handler::new(transaction_sender, receipt_receiver);

    let bank = Bank::load().await.expect("unable to load bank file");

    let _bank_thread = tokio::task::spawn(bank_loop(bank, transaction_receiver, receipt_sender));

    if let Err(err) = discord::run(event_handler, discord_token).await {
        error!("error running discord client: {:?}", err);
    }
}

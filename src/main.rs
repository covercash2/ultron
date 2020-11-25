#![feature(async_closure)]
use coins::TransactionSender;
use log::error;

use tokio::sync::mpsc::{channel, Receiver, Sender};

mod logger;

mod coins;
mod commands;
mod error;

mod discord;
mod gambling;
mod tokens;

use coins::{bank_loop, Bank, Receipt, Transaction};
use discord::Handler;
use tokens::load_token;

#[tokio::main]
async fn main() {
    logger::init();

    let discord_token = load_token(tokens::DISCORD_TOKEN).expect("unable to load discord token");

    let (transaction_sender, transaction_receiver): (Sender<Transaction>, Receiver<Transaction>) =
        channel(100);
    let (receipt_sender, receipt_receiver): (Sender<Receipt>, Receiver<Receipt>) = channel(100);

    let bank_channel = TransactionSender::new(transaction_sender, receipt_receiver);

    let event_handler = Handler::new(bank_channel);

    let bank = Bank::load().await.expect("unable to load bank file");

    let _bank_thread = tokio::task::spawn(bank_loop(bank, transaction_receiver, receipt_sender));

    if let Err(err) = discord::run(event_handler, discord_token).await {
        error!("error running discord client: {:?}", err);
    }
}

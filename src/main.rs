use log::error;
use log::info;
use pretty_env_logger;

use tokio::sync::mpsc::{channel, Receiver, Sender};

mod coins;
mod commands;
mod error;

mod discord;
mod github;

mod tokens;

use coins::{bank_loop, Receipt, Transaction};
use discord::Handler;
use tokens::load_token;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let _github_token = load_token(tokens::GITHUB_TOKEN).expect("unable to load github token");
    let discord_token = load_token(tokens::DISCORD_TOKEN).expect("unable to load discord token");

    let (transaction_sender, transaction_receiver): (Sender<Transaction>, Receiver<Transaction>) =
        channel(100);
    let (receipt_sender, mut receipt_receiver): (Sender<Receipt>, Receiver<Receipt>) = channel(100);

    let event_handler = Handler::new(transaction_sender);

    let _bank_thread = tokio::task::spawn(bank_loop(
        Default::default(),
        transaction_receiver,
        receipt_sender,
    ));

    let _receipt_printer = tokio::task::spawn(async move {
	if let Some(receipt) = receipt_receiver.recv().await {
	    info!("receipt: {:?}", receipt);
	}
    });

    if let Err(err) = discord::run(event_handler, discord_token).await {
        error!("error running discord client: {:?}", err);
    }
}

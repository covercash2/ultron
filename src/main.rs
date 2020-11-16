use log::error;
use pretty_env_logger;

use tokio::sync::mpsc::{channel, Sender, Receiver};

mod coins;
mod commands;
mod error;

mod discord;
mod github;

mod tokens;

use coins::{bank_loop, Transaction};
use discord::Handler;
use tokens::load_token;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let _github_token = load_token(tokens::GITHUB_TOKEN)
        .expect("unable to load github token");
    let discord_token = load_token(tokens::DISCORD_TOKEN)
        .expect("unable to load discord token");

    let (sender, receiver): (Sender<Transaction>, Receiver<Transaction>) = channel(100);

    let event_handler = Handler::new(sender);

    let _bank_thread = tokio::task::spawn(bank_loop(Default::default(), receiver));

    if let Err(err) = discord::run(event_handler, discord_token).await {
	error!("error running discord client: {:?}", err);
    }

}

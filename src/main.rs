use log::error;
use pretty_env_logger;

mod commands;
mod discord;
mod error;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    if let Err(err) = discord::run().await {
	error!("error running discord client: {:?}", err);
    }
}

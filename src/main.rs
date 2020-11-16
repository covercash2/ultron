use log::error;
use pretty_env_logger;

mod commands;
mod error;

mod discord;
mod github;

mod tokens;

use tokens::load_token;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // TODO remove or do something with this
    //let github_token = load_token(tokens::GITHUB_TOKEN).expect("unable to load github token");
    let discord_token = load_token(tokens::DISCORD_TOKEN).expect("unable to load discord token");

    if let Err(err) = discord::run(discord_token).await {
        error!("error running discord client: {:?}", err);
    }
}

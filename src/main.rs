#![feature(async_closure)]
#![feature(str_split_once)]
use std::{env, path::PathBuf};

use db::Db;
use dotenv::dotenv;
use log::*;

use serenity::http::Http;

mod logger;

mod chat;
mod coins;
mod commands;
mod data;
mod error;
mod items;

mod copypasta;
mod discord;
mod gambling;
mod tokens;

use coins::DailyLog;
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

    let http = Http::new_with_token(&discord_token);

    let ultron_id = http
        .get_current_application_info()
        .await
        .map(|info| info.id)
        .expect("unable to get discord application info");

    let db = Db::open(&database_url).expect("unable to open database connection").into();
    let daily_log = DailyLog::load().await.expect("unable to load daily log");

    let event_handler = Handler::new(db, daily_log, ultron_id);

    if let Err(err) = discord::run(event_handler, discord_token).await {
        error!("error running discord client: {:?}", err);
    }
}

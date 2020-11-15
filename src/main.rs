use log::{debug, info, warn, error};
use pretty_env_logger;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::env;

mod commands;

use commands::{Command, Error};

struct Handler;

async fn say<T: AsRef<Http>>(
    channel: ChannelId,
    pipe: T,
    msg: impl std::fmt::Display
) {
    if let Err(err) = channel.say(pipe, msg).await {
	error!("error sending message: {:?}", err);
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {

	match msg.content.parse() {
	    Ok(Command::Ping) => {
		say(msg.channel_id, &ctx.http, "hello").await;
	    },
	    Ok(Command::About) => {
		say(
		    msg.channel_id,
		    &ctx.http,
		    "https://github.com/covercash2/ultron"
		).await;
	    },
	    Ok(Command::Announce) => {
		say(
		    msg.channel_id,
		    &ctx.http,
		    "I am always listening"
		).await;
	    },
	    Err(Error::UnknownCommand(s)) => {
		debug!("unable to parse command: {:?}", s);
	    }
	}
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let token = env::var("DISCORD_TOKEN").expect("unable load env discord token");

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .await
        .expect("unable to create client");

    if let Err(err) = client.start().await {
        error!("client error: {:?}", err);
    }
}

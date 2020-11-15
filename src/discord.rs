use log::*;
use std::env;

use serenity::http::Http;
use serenity::model::id::ChannelId;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

use crate::commands::Command;
use crate::error::{Result, Error};

pub async fn run() -> Result<()> {
    let token = env::var("DISCORD_TOKEN")?;

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .await
        .expect("unable to create client");

    client.start().await
        .map_err(Error::from)
}

struct Handler;

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

async fn say<T: AsRef<Http>>(
    channel: ChannelId,
    pipe: T,
    msg: impl std::fmt::Display
) {
    if let Err(err) = channel.say(pipe, msg).await {
	error!("error sending message: {:?}", err);
    }
}

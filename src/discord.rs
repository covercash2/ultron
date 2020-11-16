use log::*;

use serenity::http::Http;
use serenity::model::id::ChannelId;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

use crate::commands::Command;
use crate::error::{Error, Result};

pub async fn run<S: AsRef<str>>(token: S) -> Result<()> {
    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .await
        .expect("unable to create client");

    client.start().await.map_err(Error::from)
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        match msg
            .content
            .parse::<Command>()
            .and_then(|command| command.process())
        {
            Ok(command_output) => {
		say(msg.channel_id, &ctx.http, command_output).await;
	    }
            Err(err) => {
                debug!("unable to execute command: {:?}", err);
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

async fn say<T: AsRef<Http>>(channel: ChannelId, pipe: T, msg: impl std::fmt::Display) {
    if let Err(err) = channel.say(pipe, msg).await {
        error!("error sending message: {:?}", err);
    }
}

use log::*;

use serenity::http::Http;
use serenity::model::channel::Reaction;
use serenity::model::id::ChannelId;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

use tokio::sync::mpsc::Sender;

use crate::commands::Command;
use crate::error::{Error, Result};

pub async fn run<S: AsRef<str>>(handler: Handler, token: S) -> Result<()> {
    let mut client = Client::builder(&token)
        .event_handler(handler)
        .await
        .expect("unable to create client");

    client.start().await.map_err(Error::from)
}

pub struct Handler {
    coin_sender: Sender<(u64, i64)>,
}

impl Handler {
    pub fn new(coin_sender: Sender<(u64, i64)>) -> Handler {
	Handler { coin_sender }
    }

    pub async fn send_coins<U: Into<u64>>(&self, user: U, coin_num: i64) {
	let user_id = user.into();
	// TODO return error
	// if let Err(err) = self.coin_sender.send((user_id, coin_num)).await {
	//     error!("unable to send coins");
	// }
    }
}

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

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
	match reaction.emoji.as_data().as_str() {
	    "🪙" => {
		// coin added
		debug!("coin added");
		let name = reaction.user(&ctx.http).await.unwrap().name;
		debug!("user: {}", name);

		if let Some(id) = reaction.user_id {
		    self.send_coins(id, 1);
		} else {
		    warn!("no user id")
		}

	    },
	    _ => {}
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

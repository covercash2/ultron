use std::sync::Arc;

use log::*;

use serenity::http::Http;
use serenity::model::channel::Reaction;
use serenity::model::id::ChannelId;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

use tokio::sync::{Mutex, mpsc::{Receiver, Sender}};

use crate::coins::{Receipt, Transaction};
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
    coin_sender: Sender<Transaction>,
    receipt_receiver: Arc<Mutex<Receiver<Receipt>>>,
}

impl Handler {
    pub fn new(coin_sender: Sender<Transaction>, receipt_receiver: Receiver<Receipt>) -> Handler {
	let receipt_receiver = Arc::new(Mutex::new(receipt_receiver));
        Handler {
            coin_sender,
            receipt_receiver,
        }
    }

    pub async fn send_coins<U: Into<u64>>(
        &self,
        from_user: U,
        to_user: U,
        coin_num: i64,
    ) -> Result<()> {
        let from_user = from_user.into();
        let to_user = to_user.into();
        let amount = coin_num;
        let transaction = Transaction::Transfer {
            to_user,
            from_user,
            amount,
        };
        let mut sender = self.coin_sender.clone();
        sender.send(transaction).await?;
	let mut lock = self.receipt_receiver.lock().await;
	if let Some(receipt) = lock.recv().await {
	    receipt.iter().for_each(|entry| info!("entry: {:?}", entry));
	}

	Ok(())
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

                if let Some(giver_id) = reaction.user_id {
                    let author_id = reaction.message(&ctx.http).await.map(|message| {
                        info!("{} giving {} a coin", name, message.author.name);
                        message.author.id
                    });

                    match author_id {
                        Ok(id) => {
                            if let Err(err) = self.send_coins(giver_id, id, 1).await {
				error!("error sending coins: {:?}", err);
			    }
                        }
                        Err(err) => {
                            warn!("no user id found: {:?}", err);
                        }
                    }
                }
            }
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

use std::sync::Arc;

use log::*;

use serenity::http::Http;
use serenity::model::channel::Reaction;
use serenity::model::id::ChannelId;
use serenity::model::id::UserId;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};

use crate::coins::{Receipt, Transaction};
use crate::commands;
use crate::commands::Command;
use crate::error::{Error, Result};

pub async fn run<S: AsRef<str>>(handler: Handler, token: S) -> Result<()> {
    let mut client = Client::builder(&token)
        .event_handler(handler)
        .await
        .expect("unable to create client");

    client.start().await.map_err(Error::from)
}

#[derive(Debug)]
pub struct DiscordMessage<'a> {
    pub context: &'a Http,
    pub message: Message,
}

pub struct Handler {
    transaction_sender: Sender<Transaction>,
    receipt_receiver: Arc<Mutex<Receiver<Receipt>>>,
}

impl Handler {
    pub fn new(
        transaction_sender: Sender<Transaction>,
        receipt_receiver: Receiver<Receipt>,
    ) -> Handler {
        let receipt_receiver = Arc::new(Mutex::new(receipt_receiver));
        Handler {
            transaction_sender,
            receipt_receiver,
        }
    }

    pub async fn send_coins<C: Into<u64>, U: Into<u64>>(
        &self,
        channel_id: C,
        from_user: U,
        to_user: U,
        coin_num: i64,
    ) -> Result<()> {
        let channel_id = channel_id.into();
        let from_user = from_user.into();
        let to_user = to_user.into();
        let amount = coin_num;
        let transaction = Transaction::Transfer {
            channel_id,
            to_user,
            from_user,
            amount,
        };
        let mut sender = self.transaction_sender.clone();
        sender.send(transaction).await?;
        let mut lock = self.receipt_receiver.lock().await;
        if let Some(receipt) = lock.recv().await {
            receipt
                .iter()
                .for_each(|entry| debug!("entry: {:?}", entry));
        }

        Ok(())
    }

    pub async fn process_command(&self, context: &Context, command: Command) -> Result<String> {
        match command {
            Command::Help => Ok(commands::HELP.to_owned()),
            Command::Ping => Ok(commands::PING.to_owned()),
            Command::About => Ok(commands::ABOUT.to_owned()),
            Command::Announce => Ok(commands::ANNOUNCE.to_owned()),
            Command::GetAllBalances(channel_id) => {
                let mut sender = self.transaction_sender.clone();
                let transaction = Transaction::GetAllBalances(channel_id);
                sender.send(transaction).await?;
                let mut lock = self.receipt_receiver.lock().await;
                if let Some(receipt) = lock.recv().await {
                    let mut output = String::new();
                    for (id, amount) in receipt.iter() {
                        let user_id: UserId = (*id).into();
                        let name = user_id.to_user(&context.http).await?.name;
                        output.push_str(&format!("{:15}#{:06}\n", name, amount));
                    }
                    Ok(output)
                } else {
                    Err(Error::CommandProcess(
                        "unable to process GetAllBalances command".to_owned(),
                    ))
                }
            }
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let discord_message = DiscordMessage {
            context: &ctx.http,
            message: msg.clone(),
        };

        let command = match Command::parse_message(discord_message).await {
            Ok(command) => command,
            Err(err) => {
                warn!("unable to parse command: {:?}", err);
                return;
            }
        };

        let output = match self.process_command(&ctx, command).await {
            Ok(output) => output,
            Err(err) => {
                error!("unable to process command: {:?}", err);
                return;
            }
        };

        say(msg.channel_id, &ctx.http, output).await;
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
                            if let Err(err) =
                                self.send_coins(reaction.channel_id, giver_id, id, 1).await
                            {
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

// TODO return result
async fn say<T: AsRef<Http>>(channel: ChannelId, pipe: T, msg: impl std::fmt::Display) {
    if let Err(err) = channel.say(pipe, msg).await {
        error!("error sending message: {:?}", err);
    }
}

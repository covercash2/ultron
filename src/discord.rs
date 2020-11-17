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

/// Run the main thread for the chat client.
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

/// This struct is the main handler for the [`serenity`] Discord API crate.
/// It communicates with the bank thread though the `transaction_sender` and
/// `receipt_receiver` [`tokio::sync::mpsc`] channels.
/// The `receipt_receiver` needs to be wrapped in a `Mutex` since [`Receiver`]s are not
/// thread-safe; additionally, automatic reference counting ([`Arc`]) is used to get
/// a mutable reference behind an immutable `Handler`.
pub struct Handler {
    transaction_sender: Sender<Transaction>,
    // receivers aren't thread safe, so we need some boxes here
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

    /// Process the command, performing any necessary IO operations
    pub async fn process_command(&self, context: &Context, command: Command) -> Result<Option<String>> {
        match command {
            Command::Help => Ok(Some(commands::HELP.to_owned())),
            Command::Ping => Ok(Some(commands::PING.to_owned())),
            Command::About => Ok(Some(commands::ABOUT.to_owned())),
            Command::Announce => Ok(Some(commands::ANNOUNCE.to_owned())),
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
                    Ok(Some(output))
                } else {
                    Err(Error::CommandProcess(
                        "unable to process GetAllBalances command".to_owned(),
                    ))
                }
            }
            Command::Tip { channel_id, from_user, to_user } => {
		let mut sender = self.transaction_sender.clone();
		let transaction = Transaction::Tip {
		    channel_id,
		    from_user,
		    to_user
		};
		sender.send(transaction).await?;
		let mut lock = self.receipt_receiver.lock().await;
		if let Some(receipt) = lock.recv().await {
		    info!("tip processed: {:?}", receipt);
		}
		Ok(None)
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
            Ok(Some(output)) => output,
	    Ok(None) => {
		debug!("command finished with no output");
		return;
	    },
            Err(err) => {
                error!("unable to process command: {:?}", err);
                return;
            }
        };

        say(msg.channel_id, &ctx.http, output).await;
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        let command = match Command::parse_reaction(&ctx, reaction).await {
            Ok(command) => command,
            Err(err) => {
                warn!("unable to parse reaction: {:?}", err);
                return;
            }
        };

	// no reacts need output right now
        let output = match self.process_command(&ctx, command).await {
            Ok(Some(output)) => output,
	    Ok(None) => {
		debug!("command finished with no output");
		return;
	    },
            Err(err) => {
                error!("unable to process command: {:?}", err);
                return;
            }
        };

	info!("react output: {}", output);
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

/// Use the [`serenity`] Discord API crate to send a message accross a channel
// TODO return result
async fn say<T: AsRef<Http>>(channel: ChannelId, pipe: T, msg: impl std::fmt::Display) {
    if let Err(err) = channel.say(pipe, msg).await {
        error!("error sending message: {:?}", err);
    }
}

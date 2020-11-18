use std::sync::Arc;

use log::*;

use serenity::http::Http;
use serenity::model::channel::Reaction;
use serenity::model::id::ChannelId;
use serenity::model::id::UserId;
use serenity::utils::Colour;
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

mod messages;

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

#[derive(Debug)]
pub enum Output {
    Say(String),
    Help,
}

impl From<String> for Output {
    fn from(s: String) -> Output {
	Output::Say(s)
    }
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

    /// Send a transaction to the bank thread.
    /// Returns output to say in chat.
    pub async fn send_transaction(
        &self,
        transaction: Transaction,
    ) -> Result<Receipt> {
        let mut sender = self.transaction_sender.clone();
        sender.send(transaction).await?;
        let mut lock = self.receipt_receiver.lock().await;
        if let Some(receipt) = lock.recv().await {
            Ok(receipt)
        } else {
            Err(Error::TransactionReceipt)
        }
    }

    /// Process the command, performing any necessary IO operations
    pub async fn process_command(
        &self,
        context: &Context,
        command: Command,
    ) -> Result<Option<Output>> {
        match command {
            Command::Help => Ok(Some(Output::Help)),
            Command::Ping => Ok(Some(Output::Say(commands::PING.to_owned()))),
            Command::About => Ok(Some(Output::Say(commands::ABOUT.to_owned()))),
            Command::Announce => Ok(Some(Output::Say(commands::ANNOUNCE.to_owned()))),
            Command::Coin(transaction) => {
                let receipt = self.send_transaction(transaction).await?;
		self.process_receipt(context, receipt).await
            }
        }
    }

    pub async fn process_receipt(
        &self,
        context: &Context,
        mut receipt: Receipt,
    ) -> Result<Option<Output>> {
        match receipt.transaction {
            Transaction::GetAllBalances(_user_id) => {
                receipt
                    .account_results
                    .sort_by(|(_, amount0), (_, amount1)| amount1.cmp(amount0));
                let mut output = String::new();
                for (id, amount) in receipt.iter() {
                    let user_id: UserId = (*id).into();
                    let name = user_id.to_user(&context.http).await?.name;
                    output.push_str(&format!("`{:04}`🪙\t{}\n", amount, name));
                }
                Ok(Some(output.into()))
            }
            Transaction::Transfer { .. } => {
                debug!("transfer complete");
                Ok(None)
            }
            Transaction::Tip { .. } => {
                debug!("tip complete");
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        // channel for logging
        let channel_id = msg.channel_id.clone();

        let command = match Command::parse_message(&ctx, msg).await {
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
            }
            Err(err) => {
                error!("unable to process command: {:?}", err);
                return;
            }
        };

	match output {
	    Output::Say(string) => {
		if let Err(err) = messages::say(channel_id, &ctx.http, string).await {
		    error!("error sending message: {:?}", err);
		}
	    }
	    Output::Help => {
		if let Err(err) = messages::help_message(channel_id, &ctx.http).await {
		    error!("error sending help message: {:?}", err);
		}
	    }
	}
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
            }
            Err(err) => {
                error!("unable to process command: {:?}", err);
                return;
            }
        };

        info!("react output: {:?}", output);
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

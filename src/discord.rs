use std::sync::Arc;

use log::*;

use chrono::{DateTime, Utc};

use serenity::http::Http;
use serenity::model::channel::Reaction;
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

use crate::coins::{Receipt, Transaction, TransactionStatus};
use crate::commands::{self, Command};
use crate::error::{Error, Result};
use crate::gambling::GambleOutput;

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
    DailyResponse,
    BadDailyResponse {
        next_epoch: DateTime<Utc>,
    },
    TransferSuccess {
        to_user: u64,
        to_balance: i64,
        from_user: u64,
        from_balance: i64,
        amount: i64,
    },
    Gamble(GambleOutput),
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
    /// ultron's user id
    user_id: Arc<Mutex<Option<UserId>>>,
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
        let user_id = Default::default();
        Handler {
            user_id,
            transaction_sender,
            receipt_receiver,
        }
    }

    async fn ultron_id(&self) -> Result<u64> {
        if let Some(id) = self.user_id.lock().await.as_ref() {
            Ok(*id.as_u64())
        } else {
            Err(Error::ServerState("ultron's id is not loaded".to_owned()))
        }
    }

    /// Send a transaction to the bank thread.
    /// Returns output to say in chat.
    pub async fn send_transaction(&self, transaction: Transaction) -> Result<Receipt> {
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
        channel_id: u64,
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
            Command::Gamble(mut gamble) => gamble
                .play()
                .map(|gamble_output| Some(Output::Gamble(gamble_output)))
                .map_err(Into::into),
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
            Transaction::Transfer {
                from_user,
                to_user,
                amount,
                ..
            } => {
                debug!("transfer complete");

                let to_balance = *receipt
                    .account_results
                    .iter()
                    .find(|(user_id, _balance)| user_id == &to_user)
                    .map(|(_user_id, balance)| balance)
                    .ok_or(Error::ReceiptProcess(
                        "unable to find sender account in transaction receipt".to_owned(),
                    ))?;
                let from_balance = *receipt
                    .account_results
                    .iter()
                    .find(|(user_id, _balance)| user_id == &from_user)
                    .map(|(_user_id, balance)| balance)
                    .ok_or(Error::ReceiptProcess(
                        "unable to find receiver account in transaction receipt".to_owned(),
                    ))?;

                Ok(Some(Output::TransferSuccess {
                    to_user,
                    to_balance,
                    from_user,
                    from_balance,
                    amount,
                }))
            }
            Transaction::Tip { .. } => {
                match receipt.status {
                    TransactionStatus::Complete => {
                        debug!("tip complete");
                        Ok(None)
                    }
                    TransactionStatus::SelfTip => {
                        // TODO chastize
                        Err(Error::TransactionFailed(format!(
                            "user tried to tip themselves: {:?}",
                            receipt
                        )))
                    }
                    _ => Err(Error::TransactionFailed(format!(
                        "unexpected transaction status: {:?}",
                        receipt
                    ))),
                }
            }
            Transaction::Daily { .. } => {
                match receipt.status {
                    TransactionStatus::Complete => {
                        debug!("daily complete");
                        Ok(Some(Output::DailyResponse))
                    }
                    TransactionStatus::BadDailyRequest { next_epoch } => {
                        // bad daily request
                        info!("bad daily request: {:?}", receipt);
                        // TODO chastize
                        Ok(Some(Output::BadDailyResponse { next_epoch }))
                    }
                    _ => Err(Error::TransactionFailed(format!(
                        "unexpected transaction status: {:?}",
                        receipt
                    ))),
                }
            }
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if let Some(user_id) = self.user_id.lock().await.as_ref() {
            if &msg.author.id == user_id {
                debug!("ignoring message sent by ultron");
                return;
            }
        }

        // channel for logging
        let channel_id = msg.channel_id.clone();

        let command = match Command::parse_message(&ctx, msg).await {
            Ok(command) => command,
            Err(err) => {
                warn!("unable to parse command: {:?}", err);
                return;
            }
        };

        let output = match self
            .process_command(*channel_id.as_u64(), &ctx, command)
            .await
        {
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
                debug!("sending string to discord: {}", string);
                if let Err(err) = messages::say(channel_id, &ctx.http, string).await {
                    error!("error sending message: {:?}", err);
                }
            }
            Output::Help => {
                debug!("sending help message to discord");
                if let Err(err) = messages::help_message(channel_id, &ctx.http).await {
                    error!("error sending help message: {:?}", err);
                }
            }
            Output::BadDailyResponse { next_epoch } => {
                debug!(
                    "responding to bad daily request: next epoch -- {:?}",
                    next_epoch
                );
                if let Err(err) =
                    messages::bad_daily_response(channel_id, &ctx.http, next_epoch).await
                {
                    error!("error sending bad daily response message: {:?}", err);
                }
            }
            Output::DailyResponse => {
                debug!("responding to daily request");
                if let Err(err) = messages::daily_response(channel_id, &ctx.http).await {
                    error!("error sending daily confirmation message: {:?}", err);
                }
            }
            Output::TransferSuccess {
                to_user,
                to_balance,
                from_user,
                from_balance,
                amount,
            } => {
                debug!("responding to successful coin transfer");

                if let Err(err) = messages::transfer_success(
                    channel_id,
                    &ctx.http,
                    from_user,
                    from_balance,
                    to_user,
                    to_balance,
                    amount,
                )
                .await
                {
                    error!("error sending transfer success message: {:?}", err);
                }
            }
            Output::Gamble(gamble_output) => {
                debug!("responding to gamble");

		// TODO get user balance
		let player_balance = -1;

                if let Err(err) =
                    messages::gamble_output(channel_id, &ctx.http, player_balance, gamble_output).await
                {
                    error!("error sending gamble output: {:?}", err);
                }
            }
        }
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        let channel_id = *reaction.channel_id.as_u64();

        let command = match Command::parse_reaction(&ctx, reaction).await {
            Ok(command) => command,
            Err(err) => {
                warn!("unable to parse reaction: {:?}", err);
                return;
            }
        };

        // no reacts need output right now
        let output = match self.process_command(channel_id, &ctx, command).await {
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
        // set user id for ultron
        self.user_id.lock().await.replace(ready.user.id);
        info!("{} is connected!", ready.user.name);
    }
}

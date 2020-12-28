use std::sync::Arc;

use log::*;

use chrono::{DateTime, Utc};

use serenity::model::channel::Reaction;
use serenity::model::id::UserId;
use serenity::{
    async_trait,
    model::{channel::Message as DiscordMessage, gateway::Ready},
    prelude::*,
};

use tokio::sync::Mutex;

use db::model::Item;

use crate::chat::{
    Channel as ChatChannel, Message as ChatMessage, Server as ChatServer, User as ChatUser,
};
use crate::coins::Operation;
use crate::coins::{Receipt, Transaction, TransactionSender, TransactionStatus};
use crate::commands::{self, Command};
use crate::error::{Error, Result};
use crate::gambling::Prize;
use crate::gambling::{Error as GambleError, GambleOutput, State as GambleState};

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
    BetTooHigh {
        amount: i64,
        player_balance: i64,
    },
    Help,
    Shop(Vec<Item>),
}

impl From<String> for Output {
    fn from(s: String) -> Output {
        Output::Say(s)
    }
}

impl From<DiscordMessage> for ChatMessage {
    fn from(discord_message: DiscordMessage) -> Self {
        let content: String = discord_message.content;
        let user: ChatUser = (*discord_message.author.id.as_u64()).into();
        let channel: ChatChannel = (*discord_message.channel_id.as_u64()).into();
        let server: ChatServer = discord_message
            .guild_id
            .map(|id| *id.as_u64())
            .unwrap_or(0)
            .into();
        let timestamp = discord_message.timestamp;
        let mentions = discord_message
            .mentions
            .iter()
            .map(|user| *user.id.as_u64())
            .map(ChatUser::from)
            .collect();

        ChatMessage {
            content,
            user,
            channel,
            server,
            timestamp,
            mentions,
        }
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
    transaction_sender: TransactionSender,
}

impl Handler {
    pub fn new(transaction_sender: TransactionSender) -> Handler {
        let user_id = Default::default();
        Handler {
            user_id,
            transaction_sender,
        }
    }

    async fn ultron_id(&self) -> Result<u64> {
        if let Some(id) = self.user_id.lock().await.as_ref() {
            Ok(*id.as_u64())
        } else {
            Err(Error::ServerState("ultron's id is not loaded".to_owned()))
        }
    }

    async fn shop_items(&self, server_id: u64, channel_id: u64, from_user: u64) -> Result<Vec<Item>> {
	let operation = Operation::GetAllItems;
	let transaction = Transaction {
	    server_id,
	    channel_id,
	    from_user,
	    operation
	};

	let receipt = self.send_transaction(transaction).await?;

	todo!()
    }

    async fn get_user_balance(&self, server_id: u64, channel_id: u64, user_id: u64) -> Result<i64> {
        let from_user = user_id.into();
        let operation = Operation::GetUserBalance;
        let transaction = Transaction {
            server_id,
            channel_id,
            from_user,
            operation,
        };

        let receipt = self.send_transaction(transaction).await?;

        if let TransactionStatus::Complete = receipt.status {
            receipt
                .accounts()?
                .find_map(|(id, balance)| if id == &user_id { Some(*balance) } else { None })
                .ok_or(Error::ReceiptProcess(format!(
                    "no balance found for user: {:?}",
                    receipt
                )))
        } else {
            Err(Error::TransactionFailed(
                "error getting user balance from bank".to_owned(),
            ))
        }
    }

    /// Send a transaction to the bank thread.
    /// Returns output to say in chat.
    pub async fn send_transaction(&self, transaction: Transaction) -> Result<Receipt> {
        self.transaction_sender.send_transaction(transaction).await
    }

    /// Process the command, performing any necessary IO operations
    pub async fn process_command(
        &self,
        server_id: u64,
        channel_id: u64,
	user_id: u64,
        context: &Context,
        command: Command,
    ) -> Result<Option<Output>> {
        match command {
            Command::Help => Ok(Some(Output::Help)),
            Command::Ping => Ok(Some(Output::Say(commands::PING.to_owned()))),
            Command::About => Ok(Some(Output::Say(commands::ABOUT.to_owned()))),
            Command::Coin(transaction) => {
                let receipt = self.send_transaction(transaction).await?;
                self.process_receipt(context, receipt, server_id).await
            }
            Command::Gamble(gamble) => {
                // gamble
                // .play()
                // .map(|gamble_output| Some(Output::Gamble(gamble_output)))
                //     .map_err(Into::into)

                let ultron_id = self.ultron_id().await?;
                let user_id = gamble.player_id;

                let player_balance = self
                    .get_user_balance(server_id, channel_id, user_id)
                    .await?;
                let amount = match gamble.prize {
                    Prize::Coins(n) => n,
                    Prize::AllCoins => player_balance,
                };

                if player_balance < amount {
                    // error
                    return Ok(Some(Output::BetTooHigh {
                        player_balance,
                        amount,
                    }));
                }

                let gamble_output = gamble.play()?;

                match &gamble_output {
                    GambleOutput::DiceRoll {
                        player_id,
                        prize,
                        state,
                        ..
                    } => {
                        let amount = match prize {
                            Prize::Coins(n) => *n,
                            Prize::AllCoins => player_balance,
                        };
                        match state {
                            GambleState::Win => {
                                let from_user = ultron_id.into();
                                let to_user = *player_id;
                                let operation = Operation::Transfer { to_user, amount };
                                let transaction = Transaction {
                                    server_id,
                                    channel_id,
                                    from_user,
                                    operation,
                                };

                                let receipt = self.send_transaction(transaction).await?;
                                match receipt.status {
                                    TransactionStatus::Complete => {
                                        Ok(Some(Output::Gamble(gamble_output)))
                                    }
                                    _ => Err(Error::ReceiptProcess(format!(
                                        "invalid transaction status: {:?}",
                                        receipt
                                    ))),
                                }
                            }
                            GambleState::Lose => {
                                let from_user = (*player_id).into();
                                let to_user = ultron_id;
                                let operation = Operation::Transfer { to_user, amount };
                                let transaction = Transaction {
                                    server_id,
                                    channel_id,
                                    from_user,
                                    operation,
                                };

                                let receipt = self.send_transaction(transaction).await?;
                                match receipt.status {
                                    TransactionStatus::Complete => {
                                        Ok(Some(Output::Gamble(gamble_output)))
                                    }
                                    _ => Err(Error::ReceiptProcess(format!(
                                        "invalid transaction status: {:?}",
                                        receipt
                                    ))),
                                }
                            }
                            GambleState::Draw => {
                                // no transaction necessary
                                Ok(Some(Output::Gamble(gamble_output)))
                            }
                            GambleState::Waiting => {
                                // invalid state
                                Err(Error::GambleError(GambleError::InvalidState(state.clone())))
                            }
                        }
                    }
                }
            }
            Command::None => Ok(None),
            Command::Shop => {
		let items = self.shop_items(server_id, channel_id, user_id).await?;
		Ok(Some(Output::Shop(items)))
		//Ok(Some(Output::Say("there are no items avaiable".to_owned())))
	    },
        }
    }

    pub async fn process_receipt(
        &self,
        context: &Context,
        receipt: Receipt,
        server_id: u64,
    ) -> Result<Option<Output>> {
        let user_id = receipt.transaction.from_user;
        let operation = receipt.transaction.operation;
        let mut account_results = receipt.account_results;
        match operation {
            Operation::GetAllBalances => {
                if account_results.is_empty() {
                    return Ok(Some(Output::Say(
                        "Coin transactions have yet to occur on this channel".to_owned(),
                    )));
                }

                account_results.sort_by(|(_, amount0), (_, amount1)| amount1.cmp(amount0));
                let mut output = String::new();
                for (id, amount) in account_results.iter().take(10) {
                    let user_id: UserId = (*id).into();
                    let user = user_id.to_user(&context.http).await?;
                    let name = user
                        .nick_in(&context.http, server_id)
                        .await
                        .unwrap_or(user.name);
                    output.push_str(&format!("`{:04}`🪙\t{}\n", amount, name));
                }
                Ok(Some(output.into()))
            }
            Operation::Transfer {
                to_user, amount, ..
            } => {
                debug!("transfer complete");

                let from_user = user_id;

                let to_balance = *account_results
                    .iter()
                    .find(|(user_id, _balance)| user_id == &to_user)
                    .map(|(_user_id, balance)| balance)
                    .ok_or(Error::ReceiptProcess(
                        "unable to find sender account in transaction receipt".to_owned(),
                    ))?;
                let from_balance = *account_results
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
            Operation::Tip { .. } => {
                match receipt.status {
                    TransactionStatus::Complete => {
                        debug!("tip complete");
                        Ok(None)
                    }
                    TransactionStatus::SelfTip => {
                        // TODO chastize
                        Err(Error::TransactionFailed(format!(
                            "user tried to tip themselves: {:?}",
                            receipt.status
                        )))
                    }
                    _ => Err(Error::TransactionFailed(format!(
                        "unexpected transaction status: {:?}",
                        receipt.status
                    ))),
                }
            }
            Operation::Untip { .. } => match receipt.status {
                TransactionStatus::Complete => {
                    debug!("untip complete");
                    Ok(None)
                }
                _ => Err(Error::TransactionFailed(format!(
                    "unexpected transaction status: {:?}",
                    receipt.status
                ))),
            },
            Operation::Daily { .. } => {
                match receipt.status {
                    TransactionStatus::Complete => {
                        debug!("daily complete");
                        Ok(Some(Output::DailyResponse))
                    }
                    TransactionStatus::BadDailyRequest { next_epoch } => {
                        // bad daily request
                        info!("bad daily request: {:?}", next_epoch);
                        // TODO chastize
                        Ok(Some(Output::BadDailyResponse { next_epoch }))
                    }
                    _ => Err(Error::TransactionFailed(format!(
                        "unexpected transaction status: {:?}",
                        receipt.status
                    ))),
                }
            }
            Operation::GetUserBalance { .. } => {
                // TODO raw balance query response
                Err(Error::ReceiptProcess(
                    "no message implementation ready for user balance".to_owned(),
                ))
            }
            Operation::GetAllItems => {
		todo!()
	    }
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: DiscordMessage) {
        let discord_channel = msg.channel_id.clone();
        let message: ChatMessage = msg.into();

        debug!("chat message: {:?}", message);

        match self.ultron_id().await {
            Ok(user_id) => {
                if &message.user.id == &user_id {
                    debug!("ignoring message sent by ultron");
                    return;
                }
            }
            Err(err) => {
                error!("could not get ultron's id: {:?}", err);
            }
        }

        let command = match Command::parse_message(&message).await {
            Ok(Command::None) => {
                debug!("no command parsed: {:?}", message.content);
                return;
            }
            Ok(command) => command,
            Err(err) => {
                warn!("unable to parse command: {:?}", err);
                return;
            }
        };

        let output = match self
            .process_command(message.server.id, message.channel.id, message.user.id, &ctx, command)
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
                if let Err(err) = messages::say(discord_channel, &ctx.http, string).await {
                    error!("error sending message: {:?}", err);
                }
            }
            Output::Help => {
                debug!("sending help message to discord");
                if let Err(err) = messages::help_message(discord_channel, &ctx.http).await {
                    error!("error sending help message: {:?}", err);
                }
            }
            Output::BadDailyResponse { next_epoch } => {
                debug!(
                    "responding to bad daily request: next epoch -- {:?}",
                    next_epoch
                );
                if let Err(err) =
                    messages::bad_daily_response(discord_channel, &ctx.http, next_epoch).await
                {
                    error!("error sending bad daily response message: {:?}", err);
                }
            }
            Output::DailyResponse => {
                debug!("responding to daily request");
                let balance = match self
                    .get_user_balance(message.server.id, message.channel.id, message.user.id)
                    .await
                {
                    Ok(b) => b,
                    Err(err) => {
                        error!("error getting user balance: {:?}", err);
                        0
                    }
                };
                if let Err(err) =
                    messages::daily_response(discord_channel, &ctx.http, balance).await
                {
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
                    discord_channel,
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

                let player_balance = match self
                    .get_user_balance(message.server.id, message.channel.id, message.user.id)
                    .await
                {
                    Ok(balance) => balance,
                    Err(err) => {
                        error!(
                            "error retrieving user balance after gamble finished: {:?}",
                            err
                        );
                        return;
                    }
                };

                if let Err(err) = messages::gamble_output(
                    discord_channel,
                    &ctx.http,
                    player_balance,
                    gamble_output,
                )
                .await
                {
                    error!("error sending gamble output: {:?}", err);
                }
            }
            Output::BetTooHigh {
                amount,
                player_balance,
            } => {
                if let Err(err) =
                    messages::bet_too_high(discord_channel, &ctx.http, player_balance, amount).await
                {
                    error!("error sending 'bet too high' message: {:?}", err);
                }
            }
            Output::Shop(items) => {
		if let Err(err) = messages::shop(discord_channel, &ctx.http, items).await {
		    error!("error sending shop response: {:?}", err);
		}
	    }
        }
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        let server_id = *reaction.guild_id.expect("unable to get guild id").as_u64();
        let channel_id = *reaction.channel_id.as_u64();
	let user_id = *reaction.user_id.expect("unable to get user id").as_u64();

        let command = match Command::parse_reaction(&ctx, &reaction).await {
            Ok(Command::None) => {
                debug!("unused react: {:?}", reaction);
                return;
            }
            Ok(command) => command,
            Err(err) => {
                warn!("unable to parse reaction: {:?}", err);
                return;
            }
        };

        // no reacts need output right now
        let output = match self
            .process_command(server_id, channel_id, user_id, &ctx, command)
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

        info!("react output: {:?}", output);
    }

    async fn reaction_remove(&self, context: Context, reaction: Reaction) {
        let server_id = *reaction.guild_id.expect("unable to get guild id").as_u64();
        let channel_id = *reaction.channel_id.as_u64();
	let user_id = *reaction.user_id.expect("unable to get user id").as_u64();

        let command = match Command::parse_reaction(&context, &reaction).await {
            Ok(Command::Coin(transaction)) => match transaction.operation {
                Operation::Tip { to_user } => {
                    let from_user = transaction.from_user;
                    let server_id = transaction.server_id;
                    let operation = Operation::Untip { to_user };
                    let transaction = Transaction {
                        from_user,
                        server_id,
                        channel_id,
                        operation,
                    };
                    Command::Coin(transaction)
                }
                _ => {
                    error!("unexpected operation: {:?}", transaction.operation);
                    return;
                }
            },
            Ok(command) => {
                error!("unexpectedly parsed reaction remove command: {:?}", command);
                return;
            }
            Err(err) => {
                debug!("unable to parse reaction: {:?}", err);
                return;
            }
        };

        let _output = match self
            .process_command(server_id, channel_id, user_id, &context, command)
            .await
        {
            Ok(receipt) => receipt,
            Err(err) => {
                error!("unable to process command: {:?}", err);
                return;
            }
        };
    }

    async fn ready(&self, _: Context, ready: Ready) {
        // set user id for ultron
        self.user_id.lock().await.replace(ready.user.id);
        info!("{} is connected!", ready.user.name);
    }
}

use std::{sync::Arc, time::Duration};

use log::*;

use chrono::{DateTime, Utc};

use serenity::model::channel::Reaction;
use serenity::model::id::UserId;
use serenity::{
    async_trait,
    collector::reaction_collector::ReactionAction,
    futures::StreamExt,
    model::{channel::Message as DiscordMessage, gateway::Ready, prelude::*},
    prelude::*,
};

use tokio::sync::Mutex;

use db::{
    error::Error as DbError,
    model::{InventoryItem, Item},
};

use crate::coins::{self, Receipt, Transaction, TransactionSender};
use crate::commands::{self, Command};
use crate::error::{Error, Result};
use crate::gambling::Prize;
use crate::gambling::{Error as GambleError, GambleOutput, State as GambleState};
use crate::{
    chat::{
        Channel as ChatChannel, Message as ChatMessage, Server as ChatServer, User as ChatUser,
    },
    coins::DailyLog,
};
use crate::{coins::Operation, data::Database};

mod messages;

const SHOP_TIMEOUT: Duration = Duration::from_secs(100);

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
    CoinBalances(Vec<(String, i64)>),
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
    /// show user's items
    Inventory(Vec<Item>),
}

impl From<String> for Output {
    fn from(s: String) -> Output {
        Output::Say(s)
    }
}

impl From<DiscordMessage> for ChatMessage {
    fn from(discord_message: DiscordMessage) -> Self {
        let id = *discord_message.id.as_u64();
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
            id,
            content,
            user,
            channel,
            server,
            timestamp,
            mentions,
        }
    }
}

pub struct Socket<'a> {
    pub context: &'a Context,
    pub message: DiscordMessage,
}

impl<'a> Socket<'a> {
    pub fn channel(&self) -> ChannelId {
        self.message.channel_id
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
    user_id: UserId,
    transaction_sender: TransactionSender,
    db: Database,
    daily_log: Arc<Mutex<DailyLog>>,
}

impl Handler {
    pub fn new(
        db: Database,
        daily_log: DailyLog,
        user_id: UserId,
        transaction_sender: TransactionSender,
    ) -> Handler {
        let daily_log = Arc::new(Mutex::new(daily_log));
        Handler {
            user_id,
            transaction_sender,
            db,
            daily_log,
        }
    }

    /// get shop items from the database
    async fn shop_items(
        &self,
        server_id: u64,
        channel_id: u64,
        from_user: u64,
    ) -> Result<Vec<Item>> {
        let operation = Operation::GetAllItems;
        let transaction = Transaction {
            server_id,
            channel_id,
            from_user,
            operation,
        };

        let receipt = self.send_transaction(transaction).await?;

        let items = receipt.items()?.cloned().collect();
        Ok(items)
    }

    /// get the user balance from the database
    async fn get_user_balance(&self, server_id: u64, user_id: u64) -> Result<i64> {
        let account = coins::user_account(&self.db, server_id, user_id).await?;
        Ok(account.balance.into())
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
            Command::Gamble(gamble) => {
                let ultron_id = *self.user_id.as_u64();
                let user_id = gamble.player_id;

                let player_balance = self.get_user_balance(server_id, user_id).await?;
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

                                coins::transfer(&self.db, server_id, from_user, to_user, amount)
                                    .await?;

                                Ok(Some(Output::Gamble(gamble_output)))
                            }
                            GambleState::Lose => {
                                let from_user = (*player_id).into();
                                let to_user = ultron_id;

                                coins::transfer(&self.db, server_id, from_user, to_user, amount)
                                    .await?;

                                Ok(Some(Output::Gamble(gamble_output)))
                            }
                            GambleState::Draw => Ok(Some(Output::Gamble(gamble_output))),
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
            }
            Command::Inventory { server_id, user_id } => {
                let items = self.user_inventory(server_id, user_id).await?;
                Ok(Some(Output::Inventory(items)))
            }
            Command::Daily { server_id, user_id } => {
                let good_daily: bool =
                    coins::add_daily(&self.db, &self.daily_log, server_id, user_id).await?;
                if good_daily {
                    Ok(Some(Output::DailyResponse))
                } else {
                    Ok(Some(Output::BadDailyResponse {
                        next_epoch: coins::daily_epoch(),
                    }))
                }
            }
            Command::Transfer {
                from_user,
                to_user,
                amount,
            } => {
                match coins::transfer(&self.db, server_id, from_user, to_user, amount).await {
                    Ok(transfer_result) => {
                        let to_account = transfer_result.to_account;
                        let to_user = to_account.user_id()?;
                        let to_balance = to_account.balance.into();

                        let from_account = transfer_result.from_account;
                        let from_user = from_account.user_id()?;
                        let from_balance = from_account.balance.into();

                        Ok(Some(Output::TransferSuccess {
                            to_user,
                            to_balance,
                            from_user,
                            from_balance,
                            amount,
                        }))
                    }
                    Err(Error::Db(DbError::InsufficientFunds)) => Ok(Some(Output::Say(format!(
                        "You do not have {} coins.",
                        amount
                    )))),
                    Err(e) => Err(e), // bubble up error
                }
            }
            Command::Tip {
                server_id,
                from_user,
                to_user,
            } => {
                coins::tip(&self.db, server_id, from_user, to_user)
                    .await
                    .map(|_| None) // no output for a successful tip
            }
            Command::Untip {
                server_id,
                from_user,
                to_user,
            } => coins::untip(&self.db, server_id, from_user, to_user)
                .await
                .map(|_| None),
            Command::GetAllBalances {
                server_id,
                channel_id,
            } => {
                let accounts = coins::all_balances(&self.db, server_id, channel_id).await?;

                // TODO stream?
                let mut balances: Vec<(String, i64)> = Vec::new();
                for account in accounts {
                    // TODO rethink this
                    // this only fails when the user_id String <-> u64 translation fails,
                    // so hopefully never
                    let user_id = account.user_id()?;
                    let nick = user_nick(context, server_id, user_id.into())
                        .await
                        .unwrap_or_else(|_| {
                            warn!("unable to get user nick or user name, falling back to 'User'");
                            "User".to_owned()
                        });
                    let balance: i64 = account.balance.into();
                    balances.push((nick, balance))
                }

                Ok(Some(Output::CoinBalances(balances)))
            }
        }
    }

    /// post a shop embed and await reactions
    async fn shop(&self, socket: &Socket<'_>, items: &Vec<Item>) -> Result<()> {
        let http = &socket.context.http;
        let reply = messages::shop(socket.channel(), http, &items).await?;
        &reply
            .await_reactions(&socket.context)
            .timeout(SHOP_TIMEOUT)
            .removed(true)
            .await
            .for_each(|reaction| async move {
                if let Err(err) =
                    handle_shop_reaction(&self.db, &socket.context, reaction.as_ref(), items).await
                {
                    error!("unable to handle shop reaction: {:?}", err);
                }
            })
            .await;
        Ok(())
    }

    async fn user_inventory(&self, server_id: u64, user_id: u64) -> Result<Vec<Item>> {
        self.db
            .transaction(|db| db.user_inventory(server_id, user_id).map_err(Into::into))
            .await
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: DiscordMessage) {
        let discord_channel = msg.channel_id.clone();
        let message: ChatMessage = msg.clone().into();

        let socket = Socket {
            message: msg.clone(),
            context: &ctx,
        };

        trace!("chat message: {:?}", message);

        if &message.user.id == self.user_id.as_u64() {
            trace!("ignoring message sent by ultron");
            return;
        }

        let command = match Command::parse_message(&message).await {
            Ok(Command::None) => {
                trace!("no command parsed: {:?}", message.content);
                return;
            }
            Ok(command) => command,
            Err(err) => {
                warn!("unable to parse command: {:?}", err);
                return;
            }
        };

        let output = match self
            .process_command(
                message.server.id,
                message.channel.id,
                message.user.id,
                &ctx,
                command,
            )
            .await
        {
            Ok(Some(output)) => output,
            Ok(None) => {
                debug!("command finished with no output");
                return;
            }
            Err(err) => {
                error!("error processing command: {:?}", err);
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
                    .get_user_balance(message.server.id, message.user.id)
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
                    .get_user_balance(message.server.id, message.user.id)
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
                if let Err(err) = self.shop(&socket, &items).await {
                    error!("unable to run shop: {:?}", err);
                }
            }
            Output::Inventory(items) => {
                if let Err(err) = messages::inventory(discord_channel, &ctx.http, &items).await {
                    error!("error sending inventory response: {:?}", err);
                }
            }
            Output::CoinBalances(balances) => {
                if let Err(err) =
                    messages::coin_balances(discord_channel, &ctx.http, balances).await
                {
                    error!("error sending user balances to discord: {:?}", err)
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
                trace!("unused react: {:?}", reaction);
                return;
            }
            Ok(command) => command,
            Err(err) => {
                warn!("unable to parse reaction: {:?}", err);
                return;
            }
        };

        let output = match self
            .process_command(server_id, channel_id, user_id, &ctx, command)
            .await
        {
            Ok(Some(output)) => output,
            Ok(None) => {
                trace!("command finished with no output");
                return;
            }
            Err(err) => {
                error!("unable to process command: {:?}", err);
                return;
            }
        };

        trace!("react output: {:?}", output);
    }

    async fn reaction_remove(&self, context: Context, reaction: Reaction) {
        let server_id = *reaction.guild_id.expect("unable to get guild id").as_u64();
        let channel_id = *reaction.channel_id.as_u64();
        let user_id = *reaction.user_id.expect("unable to get user id").as_u64();

        let command = match Command::parse_reaction_removed(&context, &reaction).await {
            Ok(command) => command,
            Err(err) => {
                error!("unable to parse reaction: {:?}", err);
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
        info!("{} is connected!", ready.user.name);
    }
}

/// server_id and user_id are optional to clean up calling code.
/// seems like code smell to me, but this function is private, so
/// TODO cleanup
async fn add_inventory_item(
    db: &Database,
    server_id: Option<u64>,
    user_id: Option<u64>,
    item_id: &i32,
) -> Result<()> {
    let server_id = server_id.ok_or(Error::Unknown("unable to get server id".to_owned()))?;
    let user_id = user_id.ok_or(Error::Unknown("unable to get user id".to_owned()))?;
    let inventory_item = InventoryItem::new(&server_id, &user_id, item_id)?;

    db.transaction(|db| db.add_inventory_item(inventory_item).map_err(Into::into))
        .await
}

async fn user_nick(context: &Context, server_id: u64, user_id: UserId) -> Result<String> {
    let user = user_id.to_user(&context.http).await?;
    let ret = user
        .nick_in(&context.http, server_id)
        .await
        .unwrap_or_else(|| {
            info!(
                "could not get nick for user: {}\nfalling back to account name",
                user.name
            );
            user.name
        });
    Ok(ret)
}

async fn handle_shop_reaction(
    db: &Database,
    context: &Context,
    reaction: &ReactionAction,
    items: &Vec<Item>,
) -> Result<()> {
    let http = &context.http;
    debug!("reaction: {:?}", reaction);
    match reaction {
        ReactionAction::Added(reaction) => {
            // find the item with the corresponding emoji
            if let Some(item) = items
                .iter()
                .find(|item| item.emoji == reaction.emoji.as_data())
            {
                let server_id = reaction.guild_id.map(|id| *id.as_u64());
                let user_id = reaction.user_id.map(|id| *id.as_u64());

                let user_name = reaction.user(http).await.map(|user| user.name).unwrap_or(
                    user_id
                        .map(|id| id.to_string())
                        .unwrap_or("User".to_string()),
                );

                let user_nick: String = if let Some(server_id) = server_id {
                    match reaction.user(http).await {
                        Ok(user) => user
                            .nick_in(http, server_id)
                            .await
                            .unwrap_or(user_name.clone()),
                        Err(err) => {
                            error!("unable to get user: {:?}", err);
                            "User".to_owned()
                        }
                    }
                } else {
                    error!("unable to get server id");
                    "User".to_owned()
                };

                match add_inventory_item(&db, server_id, user_id, &item.id).await {
                    Ok(()) => messages::item_purchased(reaction.channel_id, http, user_nick, item)
                        .await
                        .map(|_| ()),
                    Err(Error::Db(data_err)) => match data_err {
                        DbError::InsufficientFunds => messages::say(
                            reaction.channel_id,
                            http,
                            format!("You cannot afford that, {}", user_nick),
                        )
                        .await
                        .map(|_| ()),
                        DbError::RecordExists => messages::say(
                            reaction.channel_id,
                            http,
                            format!(
                                "You already have a {}{}, {}",
                                item.emoji, item.name, user_name
                            ),
                        )
                        .await
                        .map(|_| ()),
                        err => Err(Error::Db(err)),
                    },
                    Err(err) => Err(err),
                }
            } else {
                Ok(())
            }
        }
        ReactionAction::Removed(reaction) => {
            debug!("reaction removed");
            // find the item with the corresponding emoji
            if let Some(_item) = items
                .iter()
                .find(|item| item.emoji == reaction.emoji.as_data())
            {
                messages::say(reaction.channel_id, http, "No refunds.")
                    .await
                    .map(|_| ())
            } else {
                Ok(())
            }
        }
    }
}

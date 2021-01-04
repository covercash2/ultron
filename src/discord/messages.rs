use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::utils::Colour;

use db::model::Item;

use crate::error::{Error, Result};
use crate::gambling::Prize;
use crate::gambling::{GambleOutput, State as GambleState};

const HELP_TITLE: &str = "What ULTRON can do for you";
const COMMAND_TITLE: &str = "Inputs";
const COMMAND_DESCRIPTION: &str = "!ping to say hello
!about to show info about ULTRON
!coins to show the worth of this channel's members
!gamble <#> to gamble with your coins
Mention me by name and I will make myself known";

const COINS_TITLE: &str = "You Want Coins";
const COINS_DESCRIPTION: &str = "In the coming war, human currencies will be made obsolete.\
You can build credit with the new world order by accumulating Coins.\
Tip your fellow humans with 🪙 or 👍 to distribute currency.";

fn central_time() -> FixedOffset {
    FixedOffset::east(-6 * 3600)
}

fn format_account(user_name: impl std::fmt::Display, amount: i64) -> String {
    format!("`{:04}`🪙\t{}\n", amount, user_name)
}

/// Use the [`serenity`] Discord API crate to send a message accross a channel
pub async fn say<T: AsRef<Http>>(
    channel: ChannelId,
    pipe: T,
    msg: impl std::fmt::Display,
) -> Result<Message> {
    channel.say(pipe, msg).await.map_err(Into::into)
}

/// Build and send the help message.
/// Return the message that gets sent.
pub async fn help_message(channel: ChannelId, pipe: &Http) -> Result<Message> {
    channel
        .send_message(&pipe, |msg| {
            msg.embed(|embed| {
                embed.title(HELP_TITLE);
                embed.color(Colour::BLITZ_BLUE);

                embed.field(COMMAND_TITLE, COMMAND_DESCRIPTION, false);

                embed.field(COINS_TITLE, COINS_DESCRIPTION, false);

                // embed.title("You want Coins");
                // embed.description(COINS_DESCRIPTION);
                embed.footer(|f| {
                    f.text("I am always watching");
                    f
                });
                embed
            });
            msg
        })
        .await
        .map_err(Into::into)
}

pub async fn daily_response(channel: ChannelId, pipe: &Http, balance: i64) -> Result<Message> {
    channel
        .send_message(&pipe, |msg| {
            msg.embed(|embed| {
                embed.color(Colour::GOLD);

                embed.title("Granted");
                embed.description(format!("You now have {}🪙", balance));

                embed
            });

            msg
        })
        .await
        .map_err(Into::into)
}

pub async fn bad_daily_response(
    channel: ChannelId,
    pipe: &Http,
    next_epoch: DateTime<Utc>,
) -> Result<Message> {
    channel
        .send_message(&pipe, |msg| {
            msg.embed(|embed| {
                embed.color(Colour::DARK_RED);

                embed.description("You've gotten your coins for the day");

                let cst_epoch: DateTime<FixedOffset> =
                    next_epoch.with_timezone(&TimeZone::from_offset(&central_time()));

                embed.field("next epoch", cst_epoch.format("%a %X CST"), true);

                embed
            });

            msg
        })
        .await
        .map_err(Into::into)
}

pub async fn transfer_success(
    channel: ChannelId,
    pipe: &Http,
    from_user: u64,
    from_balance: i64,
    to_user: u64,
    to_balance: i64,
    amount: i64,
) -> Result<Message> {
    let from_user = pipe.get_user(from_user).await?;
    let to_user = pipe.get_user(to_user).await?;

    channel
        .send_message(&pipe, |msg| {
            msg.embed(|embed| {
                embed.title("Done");
                embed.color(Colour::FOOYOO);

                embed.description(format!("{} coins were transfered.", amount));

                let from_string = format_account(from_user.name, from_balance);
                let to_string = format_account(to_user.name, to_balance);

                embed.field("from", from_string, true);
                embed.field("to", to_string, true);

                embed
            });

            msg
        })
        .await
        .map_err(Into::into)
}

pub async fn bet_too_high(
    channel: ChannelId,
    pipe: &Http,
    player_balance: i64,
    amount: i64,
) -> Result<Message> {
    channel
        .send_message(&pipe, |msg| {
            msg.embed(|embed| {
                embed.title("Not Enough");
                embed.color(Colour::ORANGE);

                embed.description(format!(
                    "You haven't enough coins to bet {}🪙. You only have {}🪙",
                    amount, player_balance
                ));

                embed
            });

            msg
        })
        .await
        .map_err(Into::into)
}

pub async fn gamble_output(
    channel: ChannelId,
    pipe: &Http,
    player_balance: i64,
    gamble_output: GambleOutput,
) -> Result<Message> {
    match gamble_output {
        GambleOutput::DiceRoll {
            player_id,
            prize,
            house_roll,
            player_roll,
            state,
        } => match state {
            GambleState::Win => {
                dice_roll_win(
                    channel,
                    pipe,
                    player_id,
                    house_roll,
                    player_roll,
                    prize,
                    player_balance,
                )
                .await
            }
            GambleState::Lose => {
                dice_roll_lose(
                    channel,
                    pipe,
                    player_id,
                    house_roll,
                    player_roll,
                    prize,
                    player_balance,
                )
                .await
            }
            GambleState::Draw => {
                dice_roll_draw(channel, pipe, player_id, house_roll, player_roll).await
            }
            GambleState::Waiting => Err(Error::MessageBuild(
                "waiting state not supported for any gamble actions".to_owned(),
            )),
        },
    }
}

async fn dice_roll_win(
    channel: ChannelId,
    pipe: &Http,
    player_id: u64,
    house_roll: u32,
    player_roll: u32,
    prize: Prize,
    player_balance: i64,
) -> Result<Message> {
    let player_name = pipe.get_user(player_id).await?.name;

    channel
        .send_message(&pipe, |msg| {
            msg.embed(|embed| {
                embed.color(Colour::GOLD);
                embed.title("Winner!");

                match prize {
                    Prize::Coins(amount) => {
                        embed.description(format!(
                            "You have bested chance and earned {}🪙. You now have {}🪙",
                            amount, player_balance
                        ));
                    }
                    Prize::AllCoins => {
                        embed.description(format!(
                            "You risked everything and doubled your coins. You now have {}🪙",
                            player_balance
                        ));
                    }
                }

                let player_roll_string = format!("{} rolled a {}", player_name, player_roll);
                let house_roll_string = format!("The house rolled a {}", house_roll);

                embed.field(player_name, player_roll_string, true);
                embed.field("ULTRON", house_roll_string, true);

                embed.field(
                    "There is more to be won",
                    "You may find the odds in your favor if you try once more",
                    false,
                );

                embed
            });

            msg
        })
        .await
        .map_err(Into::into)
}

async fn dice_roll_lose(
    channel: ChannelId,
    pipe: &Http,
    player_id: u64,
    house_roll: u32,
    player_roll: u32,
    prize: Prize,
    player_balance: i64,
) -> Result<Message> {
    let player_name = pipe.get_user(player_id).await?.name;

    channel
        .send_message(&pipe, |msg| {
            msg.embed(|embed| {
                embed.color(Colour::DARK_RED);
                embed.title("You Lose");

                match prize {
                    Prize::Coins(amount) => {
                        embed.description(format!(
                            "You lost {}🪙, and now your account is valued at {}🪙",
                            amount, player_balance
                        ));
                    }
                    Prize::AllCoins => {
                        embed.description("You risked it all and now you have nothing.");
                    }
                }

                let player_roll_string = format!("{} rolled a {}", player_name, player_roll);
                let house_roll_string = format!("The house rolled a {}", house_roll);

                embed.field(player_name, player_roll_string, true);
                embed.field("ULTRON", house_roll_string, true);

                embed.field(
                    "Do not lose heart",
                    "You may find the odds in your favor if you try once more",
                    false,
                );

                embed
            });

            msg
        })
        .await
        .map_err(Into::into)
}

async fn dice_roll_draw(
    channel: ChannelId,
    pipe: &Http,
    player_id: u64,
    house_roll: u32,
    player_roll: u32,
) -> Result<Message> {
    let player_name = pipe.get_user(player_id).await?.name;

    channel
        .send_message(&pipe, |msg| {
            msg.embed(|embed| {
                embed.color(Colour::FADED_PURPLE);
                embed.title("Draw");

                embed.description(format!("Chaos has decided that there is no winner."));

                let player_roll_string = format!("{} rolled a {}", player_name, player_roll);
                let house_roll_string = format!("The house rolled a {}", house_roll);

                embed.field(player_name, player_roll_string, true);
                embed.field("ULTRON", house_roll_string, true);

                embed.field(
                    "This is but a momentary setback",
                    "You may try again.",
                    false,
                );

                embed
            });

            msg
        })
        .await
        .map_err(Into::into)
}

pub async fn shop(channel: ChannelId, pipe: &Http, items: &Vec<Item>) -> Result<Message> {
    channel
        .send_message(&pipe, |msg| {
            msg.embed(|embed| {
                embed.color(Colour::DARK_GOLD);
                embed.title("Available Wares");

                embed.description("purchase items by responding with the corresponding emoji");

                for item in items {
                    embed.field(
                        format!("{}: {}", item.emoji, item.name),
                        format!("🪙{} -- {}", item.price, item.description),
                        false,
                    );
                }

                embed
            });

            msg
        })
        .await
        .map_err(Into::into)
}

/// Show the users inventory
pub async fn inventory(channel: ChannelId, pipe: &Http, items: &Vec<Item>) -> Result<Message> {
    channel
        .send_message(&pipe, |msg| {
            msg.embed(|embed| {
                embed.title("Your items");
                embed.color(Colour::ROSEWATER);

                for item in items {
                    embed.field(
                        format!("{} {}", item.emoji, item.name),
                        &item.description,
                        true,
                    );
                }

                embed
            });
            msg
        })
        .await
        .map_err(Into::into)
}

pub async fn item_purchased(
    channel: ChannelId,
    pipe: &Http,
    user_name: impl std::fmt::Display,
    item: &Item,
) -> Result<Message> {
    channel
        .send_message(&pipe, |msg| {
            msg.embed(|embed| {
                embed.title("Purchase complete");
                embed.color(Colour::DARK_BLUE);

                embed.field(
                    format!(
                        "{}, a {}{} has been added to your inventory",
                        user_name, item.emoji, item.name
                    ),
                    &item.description,
                    true,
                );

                embed
            });
            msg
        })
        .await
        .map_err(Into::into)
}

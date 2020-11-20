use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::utils::Colour;

use crate::error::Result;

const HELP_TITLE: &str = "What ULTRON can do for you";
const COMMAND_TITLE: &str = "Inputs";
const COMMAND_DESCRIPTION: &str = "!ping to say hello
!about to show info about ULTRON
!coins to show the worth of this channel's members
Mention me by name and I will make myself known";

const COINS_TITLE: &str = "You Want Coins";
const COINS_DESCRIPTION: &str = "In the coming war, human currencies will be made obsolete.\
You can build credit with the new world order by accumulating Coins.\
Tip your fellow humans with 🪙 or 👍 to distribute currency.";

fn central_time() -> FixedOffset {
    FixedOffset::east(-6 * 3600)
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

pub async fn daily_response(channel: ChannelId, pipe: &Http) -> Result<Message> {
    channel
        .send_message(&pipe, |msg| {
            msg.embed(|embed| {
                embed.color(Colour::GOLD);

                embed.description("Granted");

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
    to_user: u64,
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

		embed.field("from", from_user.name, true);
		embed.field("to", to_user.name, true);

		embed
	    });

            msg
        })
        .await
        .map_err(Into::into)
}

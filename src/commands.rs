use serenity::client::Context;
use serenity::model::channel::Reaction;

use crate::discord::DiscordMessage;
use crate::error::{Error, Result};

pub const HELP: &'static str = "!ping to say hello
!about to show info about ultron
mentioning ultron summons him";
pub const PING: &'static str = "hello";
pub const ABOUT: &'static str = "https://github.com/covercash2/ultron";
pub const ANNOUNCE: &'static str = "I am always listening";

type UserId = u64;
type ChannelId = u64;

/// All the possible server commands
#[derive(Debug)]
pub enum Command {
    /// Print help message
    Help,
    /// Ping the server
    Ping,
    /// Print info about this bot
    About,
    /// Announce that the bot is listening
    Announce,
    /// Get all balances for accounts in the current channel
    GetAllBalances(u64),
    /// Process a tip
    Tip {
        channel_id: ChannelId,
        from_user: UserId,
        to_user: UserId,
    },
}

impl Command {
    /// Parses messages from the [`serenity`] Discord API
    pub async fn parse_message(message: DiscordMessage<'_>) -> Result<Self> {
        let content = message.message.content.as_str();
        let channel_id = message.message.channel_id;
        match content {
            "!help" => Ok(Command::Help),
            "!ping" => Ok(Command::Ping),
            "!about" => Ok(Command::About),
            "!coins" => Ok(Command::GetAllBalances(*channel_id.as_u64())),
            _ => {
                if content.contains("ultron") {
                    Ok(Command::Announce)
                } else {
                    Err(Error::UnknownCommand(content.to_owned()))
                }
            }
        }
    }

    /// Parses an emoji reaction from the [`serenity`] Discord API
    pub async fn parse_reaction(context: &Context, reaction: Reaction) -> Result<Self> {
	let channel_id = *reaction.channel_id.as_u64();
	let to_user = *reaction.message(&context.http).await?.author.id.as_u64();
	let from_user = match reaction.user_id {
	    Some(id) => *id.as_u64(),
	    None => return Err(Error::CommandParse("no user in reaction".to_owned()))
	};

	match reaction.emoji.as_data().as_str() {
            "🪙" => { // coin
		Ok(Command::Tip {
		    channel_id,
		    from_user,
		    to_user,
		})
	    }
	    _ => {
		Err(Error::CommandParse("couldn't parse command from reaction".to_owned()))
	    }
	}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commands() {}
}

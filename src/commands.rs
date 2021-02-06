use std::convert::TryInto;

use log::*;

use serenity::client::Context;
use serenity::model::channel::Reaction;
use serenity::model::channel::ReactionType;

use crate::chat::Message;
use crate::copypasta::*;
use crate::data::UserId;
use crate::error::{Error, Result};
use crate::gambling::Prize;
use crate::gambling::{Gamble, Game};

const TIP_EMOJIS: &[&str] = &[
    "🪙", "👍", "🔥", // fire
    "UP", "CRYN", "BADASS", "LAUGH",
];

pub const PING: &'static str = "hello";
pub const ABOUT: &'static str = "https://github.com/covercash2/ultron";

/// All the possible server commands
#[derive(Debug)]
pub enum Command {
    /// Print help message
    Help,
    /// Ping the server
    Ping,
    /// Print info about this bot
    About,
    GetAllBalances {
        server_id: u64,
        channel_id: u64,
    },
    Transfer {
        from_user: u64,
        to_user: u64,
        amount: i64,
    },
    Daily {
        server_id: u64,
        user_id: u64,
    },
    Tip {
        server_id: u64,
        from_user: u64,
        to_user: u64,
    },
    Untip {
        server_id: u64,
        from_user: u64,
        to_user: u64,
    },
    Gamble(Gamble),
    /// Show available items
    Shop,
    /// Show a users items
    Inventory {
        server_id: u64,
        user_id: u64,
    },
    CopyPasta {
        text: String,
    },
    None,
}

impl Command {
    /// Parses messages from the [`serenity`] Discord API
    pub async fn parse_message(message: &Message) -> Result<Self> {
        let content = message.content.as_str();
        let server_id = message.server.id;
        let user_id = message.user.id;

        let command_content: &str = if let Some(string) = content.strip_prefix('!') {
            string
        } else {
	    trace!("not a command");
            return Ok(Command::None); // not a command
        };

        let (command_str, args_str): (&str, &str) =
            if let Some(str_tuple) = command_content.split_once(' ') {
                str_tuple
            } else {
		// command with no arguments
		(command_content, "")
            };

	match command_str {
	    "help" => Ok(Command::Help),
	    "ping" => Ok(Command::Ping),
	    "about" => Ok(Command::About),
	    "coins" => {
		let channel_id = message.channel.id;
		Ok(Command::GetAllBalances {
		    server_id,
		    channel_id,
		})
	    }
	    "daily" => {
		trace!("request daily");
		let server_id = message.server.id;
		let user_id = message.user.id;

		Ok(Command::Daily { server_id, user_id })
	    }
	    "shop" => {
		info!("shop items requested");
		Ok(Command::Shop)
	    }
	    "inventory" => {
		trace!("inventory request");
		Ok(Command::Inventory { server_id, user_id })
	    }
	    "gamble" => parse_gamble(user_id, args_str).await,
	    "copypasta" => match args_str {
		"linux" => Ok(Command::CopyPasta {
		    text: GNU_PLUS_LINUX.to_owned(),
		}),
		"googlers" => Ok(Command::CopyPasta {
		    text: GOOGLERS.to_owned(),
		}),
		"rust" => Ok(Command::CopyPasta {
		    text: RUST.to_owned(),
		}),
		"rick and morty" => Ok(Command::CopyPasta {
		    text: RICK_AND_MORTY.to_owned(),
		}),
		_ => Err(Error::CommandParse("unknown copypasta".to_owned())),
	    },
	    "give" => {
		let args: Vec<&str> = args_str.split(' ').collect();
		parse_give(&message, &args)
	    }
	    _ => Err(Error::UnknownCommand(format!(
		"unknown command: {}",
		command_str
	    ))),
	}
    }

    /// Parses an emoji reaction from the [`serenity`] Discord API
    pub async fn parse_reaction(context: &Context, reaction: &Reaction) -> Result<Self> {
        let server_id = *reaction.guild_id.expect("no guild id").as_u64();
        let to_user: UserId = *reaction.message(&context.http).await?.author.id.as_u64();
        let from_user: UserId = match reaction.user_id {
            Some(id) => *id.as_u64(),
            None => return Err(Error::CommandParse("no user in reaction".to_owned())),
        };

        let emoji_string: String = reaction_string(reaction.emoji.clone()).ok_or(
            Error::CommandParse("no name found for custom emoji".to_owned()),
        )?;

        if TIP_EMOJIS.contains(&emoji_string.as_str()) {
            debug!("tip parsed");
            Ok(Command::Tip {
                server_id,
                from_user,
                to_user,
            })
        } else {
            Ok(Command::None)
        }
    }

    /// Parses an emoji reaction from the [`serenity`] Discord API
    pub async fn parse_reaction_removed(context: &Context, reaction: &Reaction) -> Result<Self> {
        let server_id = *reaction.guild_id.expect("no guild id").as_u64();
        let to_user: UserId = *reaction.message(&context.http).await?.author.id.as_u64();
        let from_user: UserId = match reaction.user_id {
            Some(id) => *id.as_u64(),
            None => return Err(Error::CommandParse("no user in reaction".to_owned())),
        };

        let emoji_string: String = reaction_string(reaction.emoji.clone()).ok_or(
            Error::CommandParse("no name found for custom emoji".to_owned()),
        )?;

        if TIP_EMOJIS.contains(&emoji_string.as_str()) {
            debug!("untip parsed");
            Ok(Command::Untip {
                server_id,
                from_user,
                to_user,
            })
        } else {
            Ok(Command::None)
        }
    }
}

async fn parse_gamble<S: AsRef<str>>(user_id: u64, args: S) -> Result<Command> {
    let args = args.as_ref().trim();
    if args == "all" {
        debug!("gamble all command parsed");
        let game = Game::DiceRoll(10);
        let gamble = Gamble::new(user_id, Prize::AllCoins, game);
        Ok(Command::Gamble(gamble))
    } else if let Ok(amount) = args.parse::<i64>() {
        if amount > 0 {
            debug!("gamble amount: {}", amount);
            let game = Game::DiceRoll(10);
            let gamble = Gamble::new(user_id, Prize::Coins(amount), game);
            Ok(Command::Gamble(gamble))
        } else {
            debug!("some cheeky fuck entered a negative number: {}", amount);
            Err(Error::CommandParse(format!(
                "negative gamble amount: {}",
                amount
            )))
        }
    } else {
        Err(Error::CommandParse(format!(
            "unable to parse gamble args: {}",
            args
        )))
    }
}

fn parse_give(message: &Message, args: &[&str]) -> Result<Command> {
    let to_user = if message.mentions.len() == 1 {
        message.mentions[0].id
    } else if message.mentions.len() == 0 {
        return Err(Error::CommandParse(
            "no users mentioned in give command".to_owned(),
        ));
    } else {
        return Err(Error::CommandParse(
            "for now you can only give one user coins".to_owned(),
        ));
    };
    let from_user = message.user.id;
    let amount = args
        .get(1)
        .ok_or(Error::CommandParse(
            "unable to get amount argument".to_owned(),
        ))
        .and_then(|arg| {
            arg.parse::<i64>()
                .map_err(|err| {
                    Error::CommandParse(format!("command should end with amount: {:?}", err))
                })
                .and_then(|amount| {
                    if amount < 0 {
                        Err(Error::CommandParse(format!(
                            "cannot transfer a negative amount"
                        )))
                    } else {
                        Ok(amount)
                    }
                })
        })?;

    let amount: i64 = amount
        .try_into()
        .map_err(|err| Error::CommandParse(format!("amount integer overflowed: {:?}", err)))?;

    Ok(Command::Transfer {
        from_user,
        to_user,
        amount,
    })
}

fn reaction_string(reaction: ReactionType) -> Option<String> {
    match reaction {
        ReactionType::Unicode(string) => Some(string),
        ReactionType::Custom { name, .. } => name,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commands() {}
}

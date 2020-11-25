use log::*;

use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::model::channel::Reaction;
use serenity::model::channel::ReactionType;

use crate::coins::Transaction;
use crate::error::{Error, Result};
use crate::gambling::{Gamble, Game};

const TIP_EMOJIS: &[&str] = &["🪙", "👍", "UP", "CRYN", "BADASS", "LAUGH"];

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
    /// Make a coin transaction
    Coin(Transaction),
    Gamble(Gamble),
    None,
}

impl Command {
    /// Parses messages from the [`serenity`] Discord API
    pub async fn parse_message(message: &Message) -> Result<Self> {
        let content = message.content.as_str();
        let channel_id = *message.channel_id.as_u64();

        let args: Vec<&str> = if let Some(args) = content.strip_prefix('!') {
            args.split(' ').collect()
        } else {
            // command doesn't start with the control char
            return Ok(Command::None);
        };

        let command_str: &str = match args.get(0) {
            Some(command_str) => command_str,
            None => return Ok(Command::None), // content == '!'
        };

        let user_id = *message.author.id.as_u64();

        match args.len() {
            1 => match command_str {
                "help" => Ok(Command::Help),
                "ping" => Ok(Command::Ping),
                "about" => Ok(Command::About),
                "coins" => {
                    let transaction = Transaction::GetAllBalances(channel_id);
                    Ok(Command::Coin(transaction))
                }
                "daily" => {
                    info!("request daily");
                    let timestamp = message.timestamp;
                    let user_id = *message.author.id.as_u64();
                    let transaction = Transaction::Daily {
                        channel_id,
                        user_id,
                        timestamp,
                    };

                    Ok(Command::Coin(transaction))
                }
                _ => Err(Error::UnknownCommand(format!(
                    "unknown command: {}",
                    command_str
                ))),
            },
            2 => {
                let arg = args
                    .get(1)
                    .ok_or(Error::CommandParse(format!("expected 2 args: {:?}", args)))?;
                match command_str {
                    "gamble" => parse_gamble(channel_id, user_id, arg).await,
                    _ => Err(Error::UnknownCommand(format!(
                        "unknown 2 arg command: {}",
                        content
                    ))),
                }
            }
            3 => {
                let (_, args) = args.split_at(1);

                if command_str != "give" {
                    return Err(Error::UnknownCommand(format!(
                        "unknown 3 arg command: {:?}",
                        content
                    )));
                }

                let to_user = if message.mentions.len() == 1 {
                    *message.mentions[0].id.as_u64()
                } else if message.mentions.len() == 0 {
                    return Err(Error::CommandParse(
                        "no users mentioned in give command".to_owned(),
                    ));
                } else {
                    return Err(Error::CommandParse(
                        "for now you can only give one user coins".to_owned(),
                    ));
                };
                let from_user = user_id;
                let amount = args
                    .get(1)
                    .ok_or(Error::CommandParse(
                        "unable to get amount argument".to_owned(),
                    ))
                    .and_then(|arg| {
                        arg.parse::<i64>().map_err(|err| {
                            Error::CommandParse(format!(
                                "command should end with amount: {}\n{:?}",
                                content, err
                            ))
                        })
                    })?;

                let transaction = Transaction::Transfer {
                    channel_id,
                    from_user,
                    to_user,
                    amount,
                };

                Ok(Command::Coin(transaction))
            }
	    _ => {
		Err(Error::UnknownCommand(format!("command has too many args: {}", content)))
	    }
        }
    }

    /// Parses an emoji reaction from the [`serenity`] Discord API
    pub async fn parse_reaction(context: &Context, reaction: Reaction) -> Result<Self> {
        let channel_id = *reaction.channel_id.as_u64();
        let to_user = *reaction.message(&context.http).await?.author.id.as_u64();
        let from_user = match reaction.user_id {
            Some(id) => *id.as_u64(),
            None => return Err(Error::CommandParse("no user in reaction".to_owned())),
        };

        let emoji_string: String = reaction_string(reaction.emoji).ok_or(Error::CommandParse(
            "no name found for custom emoji".to_owned(),
        ))?;

        if TIP_EMOJIS.contains(&emoji_string.as_str()) {
            let transaction = Transaction::Tip {
                channel_id,
                from_user,
                to_user,
            };
            Ok(Command::Coin(transaction))
        } else {
            Err(Error::CommandParse(
                "couldn't parse command from reaction".to_owned(),
            ))
        }
    }
}

async fn parse_gamble<S: AsRef<str>>(channel_id: u64, user_id: u64, args: S) -> Result<Command> {
    let args = args.as_ref().trim();
    if args == "all" {
        debug!("gamble all command parsed");
        todo!()
    } else if let Ok(amount) = args.parse::<i64>() {
        if amount > 0 {
            debug!("gamble amount: {}", amount);
            let game = Game::DiceRoll(10);
            let gamble = Gamble::new(channel_id, user_id, amount, game);
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

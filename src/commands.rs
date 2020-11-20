use log::*;

use serenity::client::Context;
use serenity::model::channel::Message;
use serenity::model::channel::Reaction;

use crate::coins::Transaction;
use crate::error::{Error, Result};

const TIP_EMOJIS: &[&str] = &["🪙", "👍"];

pub const PING: &'static str = "hello";
pub const ABOUT: &'static str = "https://github.com/covercash2/ultron";
pub const ANNOUNCE: &'static str = "I am always listening";

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
    /// Make a coin transaction
    Coin(Transaction),
}

impl Command {
    /// Parses messages from the [`serenity`] Discord API
    pub async fn parse_message(context: &Context, message: Message) -> Result<Self> {
        let content = message.content.as_str();
        let channel_id = *message.channel_id.as_u64();
        match content {
            "!help" => Ok(Command::Help),
            "!ping" => Ok(Command::Ping),
            "!about" => Ok(Command::About),
            "!coins" => {
                let transaction = Transaction::GetAllBalances(channel_id);
                Ok(Command::Coin(transaction))
            }
            "!daily" => {
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
            _ => {
                if let Some(args) = content.strip_prefix("!give") {
                    // get the last word
                    let amount: i64 = args
                        .rsplitn(2, ' ')
                        // advance iterator
                        //(see: https://doc.rust-lang.org/std/primitive.str.html#method.rsplit_once)
                        .next()
                        .ok_or(Error::CommandParse(format!(
                            "give command should have arguments: {}",
                            content
                        )))
                        .and_then(|amount_str| {
                            debug!("parsing amount str: \"{}\"", amount_str);
                            amount_str.parse::<i64>().map_err(|err| {
                                Error::CommandParse(format!(
                                    "command should end with amount: {}\n{:?}",
                                    content, err
                                ))
                            })
                        })?;

                    if amount < 0 {
                        return Err(Error::CommandParse(format!(
                            "bad negative amount entered: {}",
                            amount
                        )));
                    }

                    let from_user = *message.author.id.as_u64();

                    if message.mentions.len() == 1 {
                        let to_user = *message.mentions[0].id.as_u64();
                        let transaction = Transaction::Transfer {
                            channel_id,
                            from_user,
                            to_user,
                            amount,
                        };

                        Ok(Command::Coin(transaction))
                    } else if message.mentions.len() == 0 {
                        Err(Error::CommandParse(
                            "no users mentioned in give command".to_owned(),
                        ))
                    } else {
                        Err(Error::CommandParse(
                            "for now you can only give one user coins".to_owned(),
                        ))
                    }
                } else if let Ok(true) = message.mentions_me(context).await {
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
            None => return Err(Error::CommandParse("no user in reaction".to_owned())),
        };

        match reaction.emoji.as_data().as_str() {
            "🪙" | "👍" => {
                // coin
                let transaction = Transaction::Tip {
                    channel_id,
                    from_user,
                    to_user,
                };
                Ok(Command::Coin(transaction))
            }
            _ => Err(Error::CommandParse(
                "couldn't parse command from reaction".to_owned(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commands() {}
}

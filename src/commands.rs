use crate::discord::DiscordMessage;
use crate::error::{Error, Result};

pub const HELP: &'static str = "!ping to say hello
!about to show info about ultron
mentioning ultron summons him";
pub const PING: &'static str = "hello";
pub const ABOUT: &'static str = "https://github.com/covercash2/ultron";
pub const ANNOUNCE: &'static str = "I am always listening";

pub enum Command {
    Help,
    Ping,
    About,
    Announce,
    GetAllBalances(u64),
}

impl Command {
    pub fn process(&self) -> Result<String> {
        Ok(match self {
            Command::Help => HELP.to_owned(),
            Command::Ping => PING.to_owned(),
            Command::About => ABOUT.to_owned(),
            Command::Announce => ANNOUNCE.to_owned(),
            _ => todo!(),
        })
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commands() {}
}

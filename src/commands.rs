use std::str::FromStr;

use crate::error::{Error, Result};

const HELP: &'static str = "!ping to say hello
!about to show info about ultron
mentioning ultron summons him";
const PING: &'static str = "hello";
const ABOUT: &'static str = "https://github.com/covercash2/ultron";
const ANNOUNCE: &'static str = "I am always listening";

pub enum Command {
    Help,
    Ping,
    About,
    Announce,
}

impl Command {
    pub fn process(&self) -> Result<String> {
        Ok(match self {
            Command::Help => HELP.to_owned(),
            Command::Ping => PING.to_owned(),
            Command::About => ABOUT.to_owned(),
            Command::Announce => ANNOUNCE.to_owned(),
        })
    }
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "!help" => Ok(Command::Help),
            "!ping" => Ok(Command::Ping),
            "!about" => Ok(Command::About),
            _ => {
                if s.contains("ultron") {
                    Ok(Command::Announce)
                } else {
                    Err(Error::UnknownCommand(s.to_owned()))
                }
            }
        }
    }
}

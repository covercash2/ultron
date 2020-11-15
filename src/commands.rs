use std::str::FromStr;

pub enum Command {
    Ping,
    About,
    Announce,
}

pub enum Error {
    UnknownCommand(String),
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
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

use std::str::FromStr;

use strum::{Display, EnumIter, EnumMessage, IntoEnumIterator as _};

use crate::{
    ApiInput, Event, EventError,
    copypasta::{copy_pasta, copy_pasta_names},
    dice::DiceRoll,
};

#[derive(thiserror::Error, Debug, PartialEq, Clone)]
pub enum CommandParseError {
    #[error("input is missing prefix {0}")]
    MissingPrefix(String),
    #[error("input is missing command {0}")]
    MissingCommand(String),
    #[error("undefined command in input {0}")]
    UndefinedCommand(String),
}

#[derive(Debug, Clone, PartialEq, strum::EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter, Display, EnumMessage))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
pub enum Command {
    #[strum_discriminants(strum(message = "make Ultron say something"))]
    Echo(String),
    #[strum_discriminants(strum(message = "roll some dice"))]
    Roll(String),
    #[strum_discriminants(strum(message = "things that bear repeating"))]
    CopyPasta(String),
    #[strum_discriminants(strum(message = "get help"))]
    Help,
}

impl Command {
    pub fn execute(self) -> Result<String, EventError> {
        tracing::debug!(command = ?self, "executing command");
        let result: String = match self {
            Command::Echo(message) => message.to_string(),
            Command::Roll(input) => {
                let dice_roll: DiceRoll = input.parse()?;
                dice_roll.to_string()
            }
            Command::Help => CommandDiscriminants::iter().fold(String::new(), |acc, command| {
                format!(
                    "{}\n✨`{}` 👉 {}",
                    acc,
                    command,
                    command.get_message().unwrap_or("oops no message")
                )
            }),
            Command::CopyPasta(input) => {
                if input == "list" {
                    let names = copy_pasta_names()
                        .into_iter()
                        .map(|name| format!("✨`{}`", name))
                        .collect::<Vec<_>>()
                        .join("\n");
                    format!("types of pasta 🍝:\n{}", names)
                } else {
                    copy_pasta(&input)
                        .map(ToString::to_string)
                        .unwrap_or("try again loser".to_string())
                }
            }
        };
        Ok(result)
    }
}

impl TryFrom<Event> for Command {
    type Error = CommandParseError;

    fn try_from(input: Event) -> Result<Self, Self::Error> {
        let input: ApiInput = input.try_into()?;

        input.parse()
    }
}

impl FromStr for Command {
    type Err = CommandParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut iterator = input.split_whitespace();

        let command = iterator
            .next()
            .ok_or(CommandParseError::MissingCommand(input.to_string()))?;

        // the rest of the input joined by spaces
        let rest = iterator.collect::<Vec<_>>().join(" ");

        match command {
            "echo" => Ok(Command::Echo(rest.to_string())),
            "roll" => Ok(Command::Roll(rest.to_string())),
            "pasta" => Ok(Command::CopyPasta(rest.to_string())),
            "help" => Ok(Command::Help),
            _ => Err(CommandParseError::UndefinedCommand(command.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_parse() {
        let command: Command = "echo hello".parse().unwrap();
        assert_eq!(command, Command::Echo("hello".to_string()));
    }

    #[test]
    fn command_parse_missing_command() {
        let command: Result<Command, CommandParseError> = "".parse();
        assert_eq!(
            command.expect_err("should fail to parse"),
            CommandParseError::MissingCommand("".to_string())
        );
    }

    #[test]
    fn command_parse_undefined_command() {
        let command: Result<Command, CommandParseError> = "undefined hello".parse();
        assert_eq!(
            command.expect_err("should fail to parse"),
            CommandParseError::UndefinedCommand("undefined hello".to_string())
        );
    }
}

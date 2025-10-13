use std::str::FromStr;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumMessage, IntoEnumIterator as _};
use tyche::dice;

use crate::{
    Response,
    copypasta::{copy_pasta, copy_pasta_names},
    dice::{DiceRollResult, DiceRoller, RollerImpl},
    event_processor::{Event, EventConsumer, EventError, EventType},
};

/// consumes [`Event`]s and produces [`Response`]s
/// based on the contents of the event.
///
/// implements the [`EventConsumer`] trait
/// to be used with the [`crate::event_processor::EventProcessor`].
///
/// see [`Command`] for the list of supported commands.
#[derive(Debug, Clone)]
pub struct CommandConsumer<TRoller> {
    pub dice_roller: DiceRoller<TRoller>,
}

impl<TRoller> CommandConsumer<TRoller>
where
    TRoller: RollerImpl,
{
    pub fn new(dice_roller: DiceRoller<TRoller>) -> Self {
        Self { dice_roller }
    }

    pub async fn consume(&self, event: Event) -> Result<String, EventError> {
        let command: Command = event.try_into()?;
        let context = CommandContext {
            dice_roller: self.dice_roller.clone(),
        };
        let response = command.execute(context)?;
        Ok(response)
    }
}

#[cfg(test)]
impl CommandConsumer<tyche::dice::roller::Max> {
    pub fn with_max_dice_roller() -> Self {
        Self::new(DiceRoller::max())
    }
}

#[async_trait::async_trait]
impl<TRoller> EventConsumer for CommandConsumer<TRoller>
where
    TRoller: RollerImpl + 'static,
{
    async fn consume_event(&self, event: Event) -> Result<Response, EventError> {
        self.consume(event).await.map(Response::PlainChat)
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Clone)]
pub enum CommandParseError {
    #[error("input is missing prefix {0}")]
    MissingPrefix(String),
    #[error("input is missing command {0}")]
    MissingCommand(String),
    #[error("undefined command in input '{command}' with args {args:?}")]
    UndefinedCommand {
        command: String,
        args: Option<String>,
    },
}

#[derive(
    Debug, Clone, PartialEq, strum::EnumDiscriminants, Serialize, Deserialize, utoipa::ToSchema,
)]
#[strum_discriminants(derive(EnumIter, Display, EnumMessage))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
pub enum Command {
    #[strum_discriminants(strum(message = "make Ultron say something"))]
    Echo(String),
    #[strum_discriminants(strum(message = "roll some dice"))]
    Roll(String),
    #[strum_discriminants(strum(message = "things that bear repeating"))]
    Copypasta(String),
    #[strum_discriminants(strum(message = "get help"))]
    Help,
}

/// context for executing a command
/// that lives for the duration of the command execution.
pub struct CommandContext<T: dice::Roller> {
    pub dice_roller: DiceRoller<T>,
}

impl Command {
    pub fn execute<TRoller>(self, context: CommandContext<TRoller>) -> Result<String, EventError>
    where
        TRoller: RollerImpl,
    {
        tracing::debug!(command = ?self, "executing command");
        let result: String = match self {
            Command::Echo(message) => message.to_string(),
            Command::Roll(input) => {
                let dice_roll = DiceRollResult::from_str(&input, context.dice_roller.clone())?;
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
            Command::Copypasta(input) => {
                if input == "list" {
                    let names = copy_pasta_names()
                        .into_iter()
                        .map(|name| format!("✨`{}`", name))
                        .collect::<Vec<_>>()
                        .join("\n");
                    format!("types of pasta 🍝:\n{}", names)
                } else {
                    copy_pasta(&input).unwrap_or("try again loser".to_string())
                }
            }
        };
        Ok(result)
    }
}

impl TryFrom<Event> for Command {
    type Error = CommandParseError;

    fn try_from(input: Event) -> Result<Self, Self::Error> {
        match input.event_type {
            EventType::Command => input.content.to_string().parse(),
            _ => Err(CommandParseError::MissingPrefix(input.content.to_string())),
        }
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
            "pasta" => Ok(Command::Copypasta(rest.to_string())),
            "help" => Ok(Command::Help),
            command => Err(CommandParseError::UndefinedCommand {
                command: command.to_string(),
                args: if rest.is_empty() { None } else { Some(rest) },
            }),
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
            CommandParseError::UndefinedCommand {
                command: "undefined".to_string(),
                args: Some("hello".to_string()),
            }
        );
    }
}

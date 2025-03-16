use std::str::FromStr;

const DEFAULT_COMMAND_PREFIX: &str = "!ultron";

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to parse command from input: {0:?}")]
    CommandParse(CommandParseError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatInput {
    content: String,
}

impl<T> From<T> for ChatInput
where
    T: ToString,
{
    fn from(content: T) -> Self {
        ChatInput {
            content: content.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    ChatInput(ChatInput),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    PlainChat(String),
}

#[derive(Debug, Clone)]
pub struct EventProcessor;

impl EventProcessor {
    pub async fn process(&self, event: Event) -> Option<Response> {
        tracing::debug!("processing event: {:?}", event);
        match event {
            Event::ChatInput(chat_input) => {
                let command: Command = chat_input.content.parse().ok()?;
                match command {
                    Command::Echo(message) => Some(Response::PlainChat(format!("you said: {}", message))),
                }
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CommandParseError {
    #[error("input is missing prefix {0}")]
    MissingPrefix(String),
    #[error("input is missing command {0}")]
    MissingCommand(String),
    #[error("undefined command in input {0}")]
    UndefinedCommand(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Echo(String),
}

impl FromStr for Command {
    type Err = CommandParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut iterator = input.split_whitespace();
        let _keyword = iterator
            .next()
            .ok_or(CommandParseError::MissingPrefix(input.to_string()))?;

        let command = iterator
            .next()
            .ok_or(CommandParseError::MissingCommand(input.to_string()))?;

        match command {
            "echo" => Ok(Command::Echo(
                iterator.fold(String::new(), |acc, x| acc + x + " "),
            )),
            _ => Err(CommandParseError::UndefinedCommand(input.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let event: ChatInput = "!ultron hello".into();
        let event: Event = Event::ChatInput(event);
        let processor = EventProcessor;
        let response = processor
            .process(event)
            .await
            .expect("should get a response");
        assert_eq!(response, Response::PlainChat("you said: hello".to_string()));
    }

    #[test]
    fn test_command_parse() {
        let command: Command = "!ultron echo hello".parse().unwrap();
        assert_eq!(command, Command::Echo("hello ".to_string()));
    }
}

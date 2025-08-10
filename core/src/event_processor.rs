use std::fmt::Display;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use tokio::sync::Mutex;
use tyche::dice::roller::FastRand;

use crate::ChatInput;
use crate::Response;
use crate::User;
use crate::command::Command;
use crate::command::CommandContext;
use crate::command::CommandParseError;
use crate::dice::DiceRoller;
use crate::dice::RollerImpl;
use crate::lm::{LanguageModel, LanguageModelError};

const ULTRON_SYSTEM_PROMPT: &str = include_str!("../../prompts/ultron.md");

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("failed to parse command from input: {0}")]
    CommandParse(#[from] CommandParseError),
    #[error("failed to parse dice roll from input: {0}")]
    DiceRollParse(#[from] crate::dice::DiceRollError),

    #[error("language model error: {0}")]
    LanguageModel(#[from] LanguageModelError),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Command,
    LanguageModel,
    Plain,
}

/// represents an event that can be processed by the bot.
/// stripped of any command prefix or control characters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Event {
    pub user: User,
    pub content: BotMessage,
    pub event_type: EventType,
}

impl Event {
    /// Creates a new event from a chat input and an event type.
    /// If the event type is `Command`, it will strip the command prefix from the content.
    pub fn new(chat_input: ChatInput, event_type: EventType) -> Result<Self, CommandParseError> {
        let user = chat_input.user.clone();
        let (content, event_type) = if event_type == EventType::Plain {
            let result = chat_input.strip_prefix();
            if let Ok(content) = result {
                (content, EventType::Command)
            } else {
                (chat_input.content.as_str(), EventType::Plain)
            }
        } else {
            (chat_input.content.as_str(), event_type)
        };

        Ok(Event {
            user,
            content: BotMessage::raw(content),
            event_type,
        })
    }
}

#[derive(Debug, Clone)]
pub struct EventProcessor<TRoller = FastRand> {
    language_model: LanguageModel,
    events: EventLog,
    pub dice_roller: DiceRoller<TRoller>,
}

#[cfg(test)]
impl Default for EventProcessor<tyche::dice::roller::Max> {
    fn default() -> Self {
        Self::new("http://localhost:11434", DiceRoller::max())
            .expect("should create default event processor")
    }
}

impl<TRoller: RollerImpl> EventProcessor<TRoller> {
    pub fn new(
        lm_endpoint: impl AsRef<str>,
        dice_roller: DiceRoller<TRoller>,
    ) -> Result<Self, LanguageModelError> {
        let language_model = LanguageModel::ollama(lm_endpoint.as_ref(), Default::default())?;

        let system_message = Event {
            user: User::System,
            content: BotMessage::raw(ULTRON_SYSTEM_PROMPT),
            event_type: EventType::LanguageModel,
        };

        let events = EventLog::new([system_message]);

        Ok(Self {
            language_model,
            events,
            dice_roller,
        })
    }
}

impl EventProcessor<FastRand> {
    pub fn with_rng(lm_endpoint: impl AsRef<str>, seed: u64) -> Result<Self, LanguageModelError> {
        let dice_roller = DiceRoller::with_rng(seed);
        Self::new(lm_endpoint, dice_roller)
    }

    pub fn with_default_rng(lm_endpoint: impl AsRef<str>) -> Result<Self, LanguageModelError> {
        let dice_roller = DiceRoller::with_default_rng();
        Self::new(lm_endpoint, dice_roller)
    }
}

impl<TRoller> EventProcessor<TRoller>
where
    TRoller: RollerImpl,
{
    pub async fn process(&self, event: impl Into<Event>) -> Result<Option<Response>, EventError> {
        let event = event.into();
        tracing::debug!(?event, "processing event");

        self.events.log_event(event.clone()).await;

        if event.user == User::Ultron {
            tracing::debug!("ignoring event from Ultron");
            return Ok(None);
        }

        match event.event_type {
            EventType::Command => {
                let command: Command = event.try_into()?;
                let result = self.command_context().and_then(move |context| {
                    tracing::debug!(?command, "executing command");
                    command.execute(context)
                })?;

                tracing::debug!("computed output: {result}");

                Ok(Some(Response::PlainChat(result)))
            }
            EventType::LanguageModel => {
                let events = self.events.get_events().await;

                let next_event = self.language_model.chat(events).await?;

                tracing::info!(
                    user = ?next_event.user,
                    "language model response"
                );

                Ok(Some(Response::Bot(next_event.content)))
            }
            EventType::Plain => {
                tracing::debug!("just plain input; don't do anything with it");
                Ok(None)
            }
        }
    }

    pub fn command_context(&self) -> Result<CommandContext<TRoller>, EventError> {
        Ok(CommandContext {
            dice_roller: self.dice_roller.clone(),
        })
    }

}

impl<T> EventProcessor<T> {
    pub async fn dump_events(&self) -> Vec<Event> {
        self.events.get_events().await
    }
}

#[derive(Debug, Clone, Default)]
pub struct EventLog {
    events: Arc<Mutex<Vec<Event>>>,
}

impl EventLog {
    pub fn new(initial_events: impl IntoIterator<Item = Event>) -> Self {
        EventLog {
            events: Arc::new(Mutex::new(initial_events.into_iter().collect())),
        }
    }

    async fn log_event(&self, event: Event) {
        let mut events = self.events.lock().await;
        events.push(event);
    }

    async fn get_events(&self) -> Vec<Event> {
        let events = self.events.lock().await;
        events.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BotMessage {
    parts: Vec<MessagePart>,
}

impl FromIterator<MessagePart> for BotMessage {
    fn from_iter<I: IntoIterator<Item = MessagePart>>(iter: I) -> Self {
        let parts = iter.into_iter().collect();
        BotMessage { parts }
    }
}

impl BotMessage {
    pub fn raw(string: impl Into<String>) -> Self {
        let part = MessagePart::Text(string.into());
        BotMessage { parts: vec![part] }
    }

    pub fn render_without_thinking_parts(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| {
                if let MessagePart::Text(text) = part {
                    Some(text.clone())
                } else {
                    None // filter out thinking parts
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl Display for BotMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rendered = self
            .parts
            .iter()
            .map(|part| match part {
                MessagePart::Thinking(thinking) => format!("<think>{}</think>", thinking),
                MessagePart::Text(text) => text.clone(),
            })
            .collect::<Vec<String>>()
            .join("\n");
        write!(f, "{}", rendered)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum MessagePart {
    Thinking(String),
    Text(String),
}

/// an iterator over a message that returns [`MessagePart`]s,
/// separating out different parts of the message,
/// crucially separating "thinking" sections from normal text.
pub struct MessagePartsIterator<'msg> {
    message: &'msg str,
    start_delim: &'msg str,
    end_delim: &'msg str,
    cursor: usize,
}

impl<'msg> MessagePartsIterator<'msg> {
    pub fn new(message: &'msg str, start_delim: &'msg str, end_delim: &'msg str) -> Self {
        Self {
            message,
            start_delim,
            end_delim,
            cursor: 0,
        }
    }
}

impl Iterator for MessagePartsIterator<'_> {
    type Item = MessagePart;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor >= self.message.len() {
            return None;
        }

        split_next_thinking_section(
            &self.message[self.cursor..],
            self.start_delim,
            self.end_delim,
        )
        .map(|(first_section, thinking_section, _rest_of_message)| {
            if !first_section.is_empty() {
                self.cursor += first_section.len();
                MessagePart::Text(first_section.to_string())
            } else {
                self.cursor +=
                    thinking_section.len() + self.start_delim.len() + self.end_delim.len();
                MessagePart::Thinking(thinking_section.to_string())
            }
        })
        .or_else(|| {
            let rest = &self.message[self.cursor..];
            if !rest.is_empty() {
                self.cursor += rest.len();
                Some(MessagePart::Text(rest.to_string()))
            } else {
                None
            }
        })
    }
}

fn split_next_thinking_section<'msg>(
    message: &'msg str,
    start_delim: &str,
    end_delim: &str,
) -> Option<(&'msg str, &'msg str, &'msg str)> {
    message
        .find(start_delim)
        .and_then(|start_index| {
            message
                .find(end_delim)
                .map(|end_index_start| end_index_start + end_delim.len())
                .map(|end_index| (start_index, end_index))
        })
        .map(|(start_index, end_index)| {
            let first_section = &message[..start_index];
            let thinking_section =
                &message[start_index + start_delim.len()..end_index - end_delim.len()];
            let rest_of_message = &message[end_index..];

            Some((first_section, thinking_section, rest_of_message))
        })
        .unwrap_or(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let event: ChatInput = ChatInput::anonymous("!ultron echo hello");
        let event: Event =
            Event::new(event, EventType::Plain).expect("should parse chat input to event");
        let processor = EventProcessor::default();
        let response = processor
            .process(event)
            .await
            .expect("echo should not error")
            .expect("should return a response");
        assert_eq!(response, Response::PlainChat("hello".to_string()));
    }

    #[test]
    fn strip_prefix() {
        let chat_input: ChatInput = ChatInput::anonymous("!ultron hello");
        let input: Event = Event::new(chat_input, EventType::Plain)
            .expect("should parse chat input to api input");
        assert_eq!(input.user, User::Anonymous);
        assert_eq!(input.content, BotMessage::raw("hello"));
    }

    #[test]
    fn split_next_thinking_section_works() {
        let message = "This is a test <think>thinking part</think> and another part.";
        let start_delim = "<think>";
        let end_delim = "</think>";
        let result = split_next_thinking_section(message, start_delim, end_delim);
        assert!(result.is_some());
        let (first_section, thinking_section, rest_of_message) = result.unwrap();
        assert_eq!(first_section, "This is a test ");
        assert_eq!(thinking_section, "thinking part");
        assert_eq!(rest_of_message, " and another part.");
    }

    #[test]
    fn thinking_iterator_works() {
        let message = "This is a test <think>thinking part</think> and another part.";
        let start_delim = "<think>";
        let end_delim = "</think>";
        let mut iterator = MessagePartsIterator::new(message, start_delim, end_delim);
        let first_part = iterator.next().unwrap();
        assert_eq!(first_part, MessagePart::Text("This is a test ".to_string()));
        let thinking_part = iterator.next().unwrap();
        assert_eq!(
            thinking_part,
            MessagePart::Thinking("thinking part".to_string())
        );
        let second_part = iterator.next().unwrap();
        assert_eq!(
            second_part,
            MessagePart::Text(" and another part.".to_string())
        );
        assert!(iterator.next().is_none());
    }

    #[test]
    fn thinking_iterator_handles_no_thinking() {
        let message = "This is a test message without thinking parts.";
        let start_delim = "<think>";
        let end_delim = "</think>";
        let mut iterator = MessagePartsIterator::new(message, start_delim, end_delim);
        let first_part = iterator.next().unwrap();
        assert_eq!(first_part, MessagePart::Text(message.to_string()));
        assert!(iterator.next().is_none());
    }

    #[test]
    fn thinking_iterator_handles_multiple_thinking_sections() {
        let message = "This is a test <think>thinking part 1</think> and another <think>thinking part 2</think>.";
        let start_delim = "<think>";
        let end_delim = "</think>";
        let mut iterator = MessagePartsIterator::new(message, start_delim, end_delim);
        let first_part = iterator.next().unwrap();
        assert_eq!(first_part, MessagePart::Text("This is a test ".to_string()));
        let thinking_part1 = iterator.next().unwrap();
        assert_eq!(
            thinking_part1,
            MessagePart::Thinking("thinking part 1".to_string())
        );
        let second_part = iterator.next().unwrap();
        assert_eq!(second_part, MessagePart::Text(" and another ".to_string()));
        let thinking_part2 = iterator.next().unwrap();
        assert_eq!(
            thinking_part2,
            MessagePart::Thinking("thinking part 2".to_string())
        );
        let last_part = iterator.next().unwrap();
        assert_eq!(last_part, MessagePart::Text(".".to_string()));
        assert!(iterator.next().is_none());
    }

    #[test]
    fn bot_message_render_without_thinking_parts() {
        let message = BotMessage {
            parts: vec![
                MessagePart::Text("This is a test".to_string()),
                MessagePart::Thinking("thinking part".to_string()),
                MessagePart::Text("and another part".to_string()),
            ],
        };
        let rendered = message.render_without_thinking_parts();
        assert_eq!(rendered, "This is a test\nand another part");
    }

    #[test]
    fn bot_message_render_without_thinking_parts_no_thinking_parts() {
        let message = BotMessage {
            parts: vec![MessagePart::Text("This is a test".to_string())],
        };
        let rendered = message.render_without_thinking_parts();
        assert_eq!(rendered, "This is a test");
    }
}

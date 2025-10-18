use std::sync::Arc;

use bon::Builder;
use futures::{StreamExt as _, stream};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::sync::Mutex;

use crate::{
    Channel, Response, User,
    chatbot::ChatInput,
    command::{CommandConsumer, CommandParseError},
    dice::DiceRoller,
    nlp::{AgentError, ChatAgent, response::LmResponse},
};

const ULTRON_SYSTEM_PROMPT: &str = include_str!("../../prompts/ultron.md");

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("failed to parse command from input: {0}")]
    CommandParse(#[from] CommandParseError),
    #[error("failed to parse dice roll from input: {0}")]
    DiceRollParse(#[from] crate::dice::DiceRollError),

    #[error("agent error: {0}")]
    Agent(#[from] AgentError),
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
#[derive(Builder, Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Event {
    pub user: User,
    pub content: LmResponse,
    pub event_type: EventType,
    pub channel: Channel,
    #[builder(default)]
    pub timestamp: EventTimestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EventTimestamp(OffsetDateTime);

impl Default for EventTimestamp {
    fn default() -> Self {
        EventTimestamp(OffsetDateTime::now_utc())
    }
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

        let event = Event::builder()
            .user(user)
            .channel(Channel::Debug)
            .content(LmResponse::raw(content))
            .event_type(event_type)
            .build();

        Ok(event)
    }
}

#[derive(Debug, Clone)]
pub struct EventProcessor {
    events: EventLog,
    consumers: EventConsumers,
}

/// a collection of [`EventConsumer`]s
/// that simplifies the [`futures::Stream`] API.
// TODO: add filters
#[derive(Debug, Clone, Default)]
struct EventConsumers(Vec<Arc<dyn EventConsumer>>);

impl EventConsumers {
    pub fn iter(&self) -> impl Iterator<Item = Arc<dyn EventConsumer>> {
        self.0.iter().cloned()
    }

    /// propagate an event to all consumers, returning a stream of results
    pub fn propagate_event(&self, event: &Event) -> impl futures::Stream<Item = EventResult> {
        let futures = self
            .iter()
            .filter(move |consumer| consumer.should_consume_event(&event))
            .map(move |consumer| {
                let event = event.clone();
                async move { consumer.consume_event(&event).await }
            });
        stream::iter(futures).buffer_unordered(4)
    }
}

/// the result of processing an event.
/// if the event was not handled, the result will be `Ok(None)`.
pub type EventResult = Result<Response, EventError>;

#[async_trait::async_trait]
pub trait EventConsumer: std::fmt::Debug + Send + Sync + 'static {
    async fn consume_event(&self, event: &Event) -> EventResult;

    fn should_consume_event(&self, _event: &Event) -> bool {
        true
    }
}

#[cfg(test)]
impl EventProcessor {
    pub async fn test() -> Self {
        Self::new()
            .with_consumer(CommandConsumer::new(DiceRoller::max()))
            .with_consumer(crate::nlp::EchoAgent)
    }
}

impl EventProcessor {
    pub fn new() -> Self {
        let system_message = Event::builder()
            .user(User::System)
            .content(LmResponse::raw(ULTRON_SYSTEM_PROMPT))
            .event_type(EventType::LanguageModel)
            .channel(Channel::Debug)
            .build();

        let events = EventLog::new([system_message]);

        Self {
            events,
            consumers: EventConsumers::default(),
        }
    }

    pub fn with_rng<TAgent>(chat_agent: TAgent, seed: u64) -> Self
    where
        TAgent: ChatAgent + 'static,
    {
        let dice_roller = DiceRoller::with_rng(seed);
        let command_consumer = CommandConsumer::new(dice_roller);
        Self::new()
            .with_consumer(command_consumer)
            .with_consumer(chat_agent)
    }

    pub fn with_default_rng() -> Self {
        let dice_roller = DiceRoller::with_default_rng();
        Self::new().with_consumer(CommandConsumer::new(dice_roller))
    }

    pub fn with_consumer<T>(mut self, consumer: T) -> Self
    where
        T: EventConsumer + 'static,
    {
        self.consumers.0.push(Arc::new(consumer));
        self
    }

    pub async fn process(&self, event: impl Into<Event>) -> Result<Vec<Response>, EventError> {
        let event = event.into();
        tracing::debug!(?event, "processing event");

        self.events.log_event(event.clone()).await;

        let event_results: Vec<EventResult> =
            Box::pin(self.consumers.propagate_event(&event).collect()).await;

        let responses: Vec<Response> = event_results
            .into_iter()
            .filter_map(|response| match response {
                Ok(resp) => Some(resp),
                Err(error) => {
                    tracing::error!(%error, "error processing event");
                    None
                }
            })
            .collect();

        Ok(responses)
    }
}

impl Default for EventProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl EventProcessor {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let event: ChatInput = ChatInput::anonymous("!ultron echo hello", Channel::Debug);
        let event: Event =
            Event::new(event, EventType::Plain).expect("should parse chat input to event");
        let processor =
            EventProcessor::new().with_consumer(CommandConsumer::new(DiceRoller::max()));
        let responses = processor
            .process(event)
            .await
            .expect("echo should not error");

        insta::assert_debug_snapshot!(responses, @r#"
        [
            PlainChat(
                "hello",
            ),
        ]
        "#);

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0], Response::PlainChat("hello".to_string()));
    }

    #[test]
    fn strip_prefix() {
        let chat_input: ChatInput = ChatInput::anonymous("!ultron hello", Channel::Debug);
        let input: Event =
            Event::new(chat_input, EventType::Plain).expect("should parse chat input to api input");
        assert_eq!(input.user, User::Anonymous);
        assert_eq!(input.content, LmResponse::raw("hello"));
    }
}

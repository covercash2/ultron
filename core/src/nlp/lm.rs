use ollama_rs::error::OllamaError;

use crate::{Channel, User, event_processor::Event, nlp::ollama::Ollama};

const KNOWN_MODELS: &[&str] = &[
    "deepseek-r1:8b",
    "qwen2.5:latest",
    "gemma2:latest",
    "llama3.2:latest",
    "starcoder2:latest",
    "mxbai-embed-large:latest",
    "mistral-nemo:latest",
    "llama3.1:latest",
];

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, schemars::JsonSchema)]
pub struct ModelName(String);

impl Default for ModelName {
    fn default() -> Self {
        KNOWN_MODELS[0].into()
    }
}

impl<T: Into<String>> From<T> for ModelName {
    fn from(name: T) -> Self {
        let name = name.into();
        Self(name)
    }
}

impl ModelName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub trait MessageFormat: std::fmt::Debug + Clone + Copy {
    fn format_event(&self, event: &Event) -> String;
}

#[derive(Debug, Clone, Copy)]
pub struct ChatFormatter;

impl MessageFormat for ChatFormatter {
    fn format_event(&self, event: &Event) -> String {
        format!("{}: {}", event.user, event.content)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlainFormatter;

impl MessageFormat for PlainFormatter {
    fn format_event(&self, event: &Event) -> String {
        event.content.to_string()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MessageFormatter {
    Chat(ChatFormatter),
    Plain(PlainFormatter),
}

impl MessageFormatter {
    pub fn chat() -> Self {
        MessageFormatter::Chat(ChatFormatter)
    }

    pub fn plain() -> Self {
        MessageFormatter::Plain(PlainFormatter)
    }

    pub fn format_event(&self, event: &Event) -> String {
        match self {
            MessageFormatter::Chat(formatter) => formatter.format_event(event),
            MessageFormatter::Plain(formatter) => formatter.format_event(event),
        }
    }
}

#[derive(bon::Builder, Debug, Clone)]
pub struct LmChatInput {
    #[builder(into)]
    pub model_name: ModelName,
    #[builder(into)]
    pub messages: Vec<(User, String)>,
    #[builder(into)]
    pub channel: Channel,
    #[builder(into)]
    pub formatter: MessageFormatter,
}

#[derive(Debug, Clone)]
pub struct LanguageModel {
    backend: LanguageModelBackend,
    model_name: ModelName,
    default_formatter: MessageFormatter,
}

impl LanguageModel {
    pub fn ollama(url: &str, model_name: ModelName) -> Result<Self, LanguageModelError> {
        let backend = LanguageModelBackend::Ollama(Ollama::new(url)?);
        Ok(Self {
            backend,
            model_name,
            default_formatter: MessageFormatter::Plain(PlainFormatter),
        })
    }

    pub async fn chat(&self, events: impl AsRef<[Event]>) -> Result<Event, LanguageModelError> {
        let messages = events
            .as_ref()
            .iter()
            .map(|event| {
                (
                    event.user.clone(),
                    self.default_formatter.format_event(event),
                )
            })
            .collect::<Vec<(User, String)>>();

        let input = LmChatInput::builder()
            .model_name(self.model_name.clone())
            .messages(messages)
            .channel(
                events
                    .as_ref()
                    .last()
                    .map(|event| event.channel)
                    .ok_or(LanguageModelError::EmptyEvent)?,
            )
            .formatter(self.default_formatter)
            .build();

        match &self.backend {
            LanguageModelBackend::Ollama(ollama) => ollama.chat(input).await,
            #[cfg(test)]
            LanguageModelBackend::Echo => {
                use crate::User;
                tracing::debug!("Echo backend is used in tests");

                let event = events
                    .as_ref()
                    .iter()
                    .last()
                    .cloned()
                    .ok_or(LanguageModelError::EmptyEvent)?
                    .clone();

                let event = Event {
                    user: User::Ultron,
                    ..event
                };

                Ok(event)
            }
        }
    }
}

#[cfg(test)]
impl Default for LanguageModel {
    fn default() -> Self {
        Self {
            backend: LanguageModelBackend::Echo,
            model_name: KNOWN_MODELS[0].into(),
            default_formatter: MessageFormatter::plain(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LanguageModelBackend {
    Ollama(Ollama),
    #[cfg(test)]
    Echo,
}

#[derive(Debug, thiserror::Error)]
pub enum LanguageModelError {
    #[error("Ollama error: {0}")]
    Ollama(#[from] OllamaError),

    #[error("MCP error: {0}")]
    McpClient(#[from] Box<crate::mcp::client::ClientError>),

    #[error("empty event provided for chat")]
    EmptyEvent,

    #[error("could not parse URL: {url}")]
    UrlParse { url: String },
}

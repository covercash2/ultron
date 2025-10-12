use ollama_rs::{
    Ollama as OllamaRs,
    generation::chat::{ChatMessage, MessageRole, request::ChatMessageRequest},
};
use reqwest::Url;

use crate::{
    User,
    event_processor::{Event, EventType},
    nlp::{
        lm::LanguageModelError,
        response::{BotMessage, MessagePartsIterator},
    },
};

#[derive(Debug, Clone)]
pub struct Ollama {
    inner: OllamaRs,
}

impl Ollama {
    pub fn new(url: &str) -> Result<Self, LanguageModelError> {
        let url: Url = url.parse().map_err(|_| LanguageModelError::UrlParse {
            url: url.to_string(),
        })?;

        tracing::debug!(?url, "creating Ollama instance with URL");

        let inner = OllamaRs::from_url(url);

        Ok(Self { inner })
    }

    pub(crate) async fn chat(
        &self,
        model_name: String,
        events: Vec<Event>,
    ) -> Result<Event, LanguageModelError> {
        let messages = events
            .into_iter()
            .map(|event| event.into())
            .collect::<Vec<_>>();

        tracing::debug!(?model_name, ?messages, "preparing chat messages for Ollama");
        let request = ChatMessageRequest::new(model_name, messages);

        tracing::debug!(?request, "sending chat messages to Ollama");

        let response = self.inner.send_chat_messages(request).await?;

        tracing::debug!(?response, "received response from Ollama");

        let user = match response.message.role {
            MessageRole::Assistant => User::Ultron,
            _ => User::Anonymous,
        };

        let content: BotMessage =
            MessagePartsIterator::new(&response.message.content, "<think>", "</think>").collect();

        let event = Event {
            user,
            content,
            event_type: EventType::LanguageModel, // Assuming event_type is not set in this conversion
        };

        Ok(event)
    }
}

impl From<Event> for ChatMessage {
    fn from(event: Event) -> Self {
        let role: MessageRole = match event.user {
            User::Ultron => MessageRole::Assistant,
            User::System => MessageRole::System,
            _ => MessageRole::User,
        };

        ChatMessage::new(role, event.content.to_string())
    }
}

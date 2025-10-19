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
        response::{LmResponse, MessagePartsIterator},
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
        events: impl AsRef<[Event]>,
    ) -> Result<Event, LanguageModelError> {
        let channel = events
            .as_ref()
            .iter()
            .last()
            .map(|e| e.channel)
            .ok_or(LanguageModelError::EmptyEvent)?;

        let messages = events
            .as_ref()
            .iter()
            .map(|event| (*event).clone().into())
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

        let content: LmResponse =
            MessagePartsIterator::new(&response.message.content, "<think>", "</think>").collect();

        tracing::debug!(%content, "parsed response, '{content}'");

        let event = Event::builder()
            .user(user)
            .content(content)
            .event_type(EventType::LanguageModel)
            .channel(channel)
            .build();

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

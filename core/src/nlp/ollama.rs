use ollama_rs::{
    Ollama as OllamaRs,
    generation::chat::{ChatMessage, MessageRole, request::ChatMessageRequest},
};
use reqwest::Url;

use crate::{
    User,
    event_processor::{Event, EventType},
    nlp::{
        lm::{LanguageModelError, LmChatInput},
        response::{MessageParts, MessagePartsIterator},
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

    pub(crate) async fn chat(&self, input: LmChatInput) -> Result<Event, LanguageModelError> {
        let channel = input.channel;

        tracing::debug!(
            model_name = ?input.model_name,
            messages = ?input.messages,
            ?channel,
            "preparing chat messages for Ollama",
        );

        let request = ChatMessageRequest::new(input.model_name.as_str().to_string(), input.into());

        tracing::debug!(?request, "sending chat messages to Ollama");

        let response = self.inner.send_chat_messages(request).await?;

        tracing::debug!(?response, "received response from Ollama");

        let user = match response.message.role {
            MessageRole::Assistant => User::Ultron,
            _ => User::Anonymous,
        };

        let content: MessageParts =
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

impl From<User> for MessageRole {
    fn from(user: User) -> Self {
        match user {
            User::Ultron => MessageRole::Assistant,
            User::System => MessageRole::System,
            _ => MessageRole::User,
        }
    }
}

impl From<Event> for ChatMessage {
    fn from(event: Event) -> Self {
        let role: MessageRole = event.user.into();

        ChatMessage::new(role, event.content.to_string())
    }
}

impl From<LmChatInput> for Vec<ChatMessage> {
    fn from(input: LmChatInput) -> Self {
        input
            .messages
            .into_iter()
            .map(|(user, content)| {
                let role: MessageRole = user.into();
                ChatMessage::new(role, content)
            })
            .collect()
    }
}

//! Natural Language Processing module.
//! encapsulates LLMs and MCP for now.

use std::{path::PathBuf, sync::Arc};

use tokio::sync::RwLock;
use tracing::instrument;

use crate::{
    Response,
    event_processor::{Event, EventConsumer, EventError, EventType},
    io::read_file_to_string,
    nlp::lm::{LanguageModel, ModelName},
};

pub mod lm;
pub mod ollama;
pub mod response;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("unable to load system prompt")]
    SystemPromptLoad(#[from] crate::error::Error),

    #[error("language model error: {0}")]
    LanguageModel(#[from] crate::nlp::lm::LanguageModelError),

    #[error("MCP client error: {0}")]
    McpClient(#[from] crate::mcp::client::ClientError),

    #[error("no events provided")]
    NoEvents,
}

pub trait ChatAgent: Clone + std::fmt::Debug + Send + Sync {
    fn chat(&self, event: &Event) -> impl Future<Output = Result<Event, AgentError>> + Send;
}

#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub struct EchoAgent;

#[cfg(test)]
impl ChatAgent for EchoAgent {
    async fn chat(&self, event: &Event) -> Result<Event, AgentError> {
        let event = event.clone();

        Ok(Event {
            user: crate::User::Ultron,
            ..event
        })
    }
}

#[async_trait::async_trait]
impl<TAgent> EventConsumer for TAgent
where
    TAgent: ChatAgent + 'static,
{
    async fn consume_event(&self, event: &Event) -> Result<Response, EventError> {
        let next_event = self.chat(event).await?;

        tracing::info!(
            user = ?next_event.user,
            "language model response"
        );

        Ok(Response::Bot(next_event.content))
    }

    fn should_consume_event(&self, event: &Event) -> bool {
        matches!(
            event.event_type,
            crate::event_processor::EventType::LanguageModel
        )
    }
}

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct ChatAgentConfig {
    pub llm_uri: String,
    pub llm_model: ModelName,
    // pub mcp_uri: String,
    pub system_prompt: PathBuf,
}

#[cfg(test)]
impl Default for ChatAgentConfig {
    fn default() -> Self {
        Self {
            llm_uri: "http://localhost:11434".to_string(),
            llm_model: "llama2".into(),
            // mcp_uri: "http://localhost:8080".to_string(),
            system_prompt: PathBuf::from("./prompts/ultron.md"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatHistory(Arc<RwLock<Vec<Event>>>);

impl ChatHistory {
    pub fn new(initial_history: impl IntoIterator<Item = Event>) -> Self {
        let chat_history = initial_history.into_iter().collect();
        Self(Arc::new(RwLock::new(chat_history)))
    }

    pub async fn append(&self, event: Event) {
        let mut history = self.0.write().await;
        history.extend([event]);
    }

    /// return a read-only snapshot of the chat history
    #[instrument(skip(self))]
    pub async fn read(&self) -> Vec<Event> {
        let history = self.0.read().await;
        history.clone()
    }
}

/// A [`ChatAgent`] that that uses a [`LanguageModel`] and [`McpClient`]
/// to process chat messages.
#[derive(Debug, Clone)]
pub struct LmChatAgent {
    // mcp: McpClient,
    language_model: LanguageModel,
    chat_history: ChatHistory,
}

impl LmChatAgent {
    pub fn new(
        language_model: LanguageModel,
        initial_history: impl IntoIterator<Item = Event>,
    ) -> Self {
        let chat_history = ChatHistory::new(initial_history);
        Self {
            // mcp,
            language_model,
            chat_history,
        }
    }

    /// Load a [`LmChatAgent`] from a [`ChatAgentConfig`].
    /// This will create a new [`McpClient`] and [`LanguageModel`].
    pub async fn load(
        ChatAgentConfig {
            llm_uri,
            llm_model,
            // mcp_uri,
            system_prompt,
        }: ChatAgentConfig,
    ) -> Result<Self, AgentError> {
        let system_prompt = read_file_to_string(system_prompt).await?;
        // let mcp = McpClient::new(&mcp_uri).await?;
        let language_model = LanguageModel::ollama(&llm_uri, llm_model)?;

        let system_prompt = Event::builder()
            .channel(crate::Channel::Debug)
            .user(crate::User::System)
            .content(system_prompt)
            .event_type(EventType::LanguageModel)
            .build();

        Ok(Self::new(language_model, [system_prompt]))
    }
}

impl ChatAgent for LmChatAgent {
    #[instrument(skip(self))]
    async fn chat(&self, event: &Event) -> Result<Event, AgentError> {
        self.chat_history.append(event.clone()).await;

        let history = self.chat_history.read().await;

        let response = self.language_model.chat(history).await?;

        self.chat_history.append(response.clone()).await;

        Ok(response)
    }
}

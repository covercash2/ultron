//! Natural Language Processing module.
//! encapsulates LLMs and MCP for now.

use crate::{
    Response,
    event_processor::{Event, EventConsumer, EventError},
    lm::{LanguageModel, ModelName},
    mcp::client::McpClient,
};

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("language model error: {0}")]
    LanguageModel(#[from] crate::lm::LanguageModelError),

    #[error("MCP client error: {0}")]
    McpClient(#[from] crate::mcp::client::ClientError),

    #[error("no events provided")]
    NoEvents,
}

pub trait ChatAgent: Clone + std::fmt::Debug + Send + Sync {
    fn chat(&self, events: Vec<Event>) -> impl Future<Output = Result<Event, AgentError>> + Send;
}

#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub struct EchoAgent;

#[cfg(test)]
impl ChatAgent for EchoAgent {
    async fn chat(&self, events: Vec<Event>) -> Result<Event, AgentError> {
        let event = events.last().cloned().ok_or(AgentError::NoEvents)?;

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
    async fn consume_event(&self, event: Event) -> Result<Option<Response>, EventError> {
        let events = vec![event];

        let next_event = self.chat(events).await?;

        tracing::info!(
            user = ?next_event.user,
            "language model response"
        );

        Ok(Some(Response::Bot(next_event.content)))
    }
}

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct ChatAgentConfig {
    pub llm_uri: String,
    pub llm_model: ModelName,
    pub mcp_uri: String,
}

#[cfg(test)]
impl Default for ChatAgentConfig {
    fn default() -> Self {
        Self {
            llm_uri: "http://localhost:11434".to_string(),
            llm_model: "llama2".into(),
            mcp_uri: "http://localhost:8080".to_string(),
        }
    }
}

/// A [`ChatAgent`] that that uses a [`LanguageModel`] and [`McpClient`]
/// to process chat messages.
#[derive(Debug, Clone)]
pub struct LmChatAgent {
    mcp: McpClient,
    language_model: LanguageModel,
}

impl LmChatAgent {
    pub fn new(mcp: McpClient, language_model: LanguageModel) -> Self {
        Self {
            mcp,
            language_model,
        }
    }

    /// Load a [`LmChatAgent`] from a [`ChatAgentConfig`].
    /// This will create a new [`McpClient`] and [`LanguageModel`].
    pub async fn load(
        ChatAgentConfig {
            llm_uri,
            llm_model,
            mcp_uri,
        }: ChatAgentConfig,
    ) -> Result<Self, AgentError> {
        let mcp = McpClient::new(&mcp_uri).await?;
        let language_model = LanguageModel::ollama(&llm_uri, llm_model)?;

        Ok(Self::new(mcp, language_model))
    }
}

impl ChatAgent for LmChatAgent {
    async fn chat(&self, events: Vec<Event>) -> Result<Event, AgentError> {
        if events.is_empty() {
            return Err(AgentError::NoEvents);
        }

        let response = self.language_model.chat(events).await?;

        Ok(response)
    }
}

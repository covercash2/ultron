use ollama_rs::error::OllamaError;

use crate::{event_processor::Event, nlp::ollama::Ollama};

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

#[derive(Debug, Clone)]
pub struct LanguageModel {
    backend: LanguageModelBackend,
    model_name: ModelName,
}

impl LanguageModel {
    pub fn ollama(url: &str, model_name: ModelName) -> Result<Self, LanguageModelError> {
        let backend = LanguageModelBackend::Ollama(Ollama::new(url)?);
        Ok(Self {
            backend,
            model_name,
        })
    }

    pub async fn chat(&self, events: Vec<Event>) -> Result<Event, LanguageModelError> {
        match &self.backend {
            LanguageModelBackend::Ollama(ollama) => {
                ollama.chat(self.model_name.0.clone(), events).await
            }
            #[cfg(test)]
            LanguageModelBackend::Echo => {
                use crate::User;
                tracing::debug!("Echo backend is used in tests");

                let event = events
                    .last()
                    .cloned()
                    .ok_or(LanguageModelError::EmptyEvent)?;

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
    McpClient(#[from] crate::mcp::client::ClientError),

    #[error("empty event provided for chat")]
    EmptyEvent,

    #[error("could not parse URL: {url}")]
    UrlParse { url: String },
}

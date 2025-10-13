use std::path::PathBuf;

use crate::command::CommandParseError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
#[error("Ultron Error")]
pub enum Error {
    #[error("failed to parse command from input: {0:?}")]
    CommandParse(CommandParseError),

    #[error("chatbot error: {0}")]
    ChatBot(Box<dyn std::error::Error + Send + Sync>),

    #[error("failed to read file {path:?}: {source}")]
    FileRead {
        source: std::io::Error,
        path: PathBuf,
    },

    #[error("failed to parse TOML file {path:?}: {source}")]
    TomlFileParse {
        source: toml::de::Error,
        path: PathBuf,
    },

    #[error("failed to generate OpenAPI doc")]
    OpenApiDocGeneration,

    #[error("failed to parse URL: {url}")]
    UrlParse { url: String },
}

#[cfg(test)]
pub struct TestError(Error);

#[cfg(test)]
impl From<TestError> for Error {
    fn from(err: TestError) -> Self {
        err.0
    }
}

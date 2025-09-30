use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
#[error("Ultron Error")]
pub enum Error {
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
}

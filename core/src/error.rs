use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
#[error("Ultron Error")]
pub enum Error {
    FileRead {
        source: std::io::Error,
        path: PathBuf,
    },
}

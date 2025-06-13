use std::path::Path;

use crate::error::{Error, Result};

pub async fn read_file_to_string(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    tokio::fs::read_to_string(path)
        .await
        .map_err(|source| Error::FileRead {
            source,
            path: path.to_path_buf(),
        })
}

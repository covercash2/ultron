use std::path::Path;

use serde::de::DeserializeOwned;
use tokio::fs;

use crate::error::{Error, Result};

pub async fn read_file_to_string(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    fs::read_to_string(path)
        .await
        .map_err(|source| Error::FileRead {
            source,
            path: path.to_path_buf(),
        })
}

pub async fn read_toml_file<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    let contents = read_file_to_string(path).await?;
    toml::from_str(&contents).map_err(|source| Error::TomlFileParse {
        source,
        path: path.to_path_buf(),
    })
}

pub mod sync {
    use std::{fs, path::Path};

    use serde::de::DeserializeOwned;

    use crate::error::{Error, Result};

    pub fn read_file_to_string(path: impl AsRef<Path>) -> Result<String> {
        let path = path.as_ref();
        fs::read_to_string(path)
            .map_err(|source| Error::FileRead {
                source,
                path: path.to_path_buf(),
            })
    }

    pub fn read_toml_file<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
        let path = path.as_ref();
        let contents = read_file_to_string(path)?;
        toml::from_str(&contents).map_err(|source| Error::TomlFileParse {
            source,
            path: path.to_path_buf(),
        })
    }
}

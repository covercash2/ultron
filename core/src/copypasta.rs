use std::{collections::HashMap, sync::OnceLock};

use crate::io::sync::read_toml_file;

const COPYPASTA_FILE: &str = "./assets/copypasta.toml";

static COPY_PASTAS: OnceLock<HashMap<String, String>> = OnceLock::new();

fn init_map() -> HashMap<String, String> {
    read_toml_file(COPYPASTA_FILE)
        .inspect_err(|error| {
            tracing::error!(
                %error,
                "unable to read copypasta file from assets",
            );
        })
        .unwrap_or_default()
}

/// get a list of all available copy pastas.
pub fn copy_pasta_names() -> Vec<String> {
    COPY_PASTAS.get_or_init(init_map).keys().cloned().collect()
}

/// get a copy pasta by its name.
pub fn copy_pasta(name: &str) -> Option<String> {
    COPY_PASTAS.get_or_init(init_map).get(name).cloned()
}

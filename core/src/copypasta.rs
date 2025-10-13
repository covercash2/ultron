use std::{collections::HashMap, sync::OnceLock};

use crate::io::sync::read_toml_file;

const COPYPASTA_FILE: &str = "../assets/copypasta.toml";

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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn test_copy_pasta_names() {
        let names: BTreeSet<String> = copy_pasta_names().into_iter().collect();

        insta::assert_json_snapshot!(names, @r#"
        [
          "googlers",
          "linux",
          "mr_robot",
          "open_source_maintainers",
          "rick_and_morty",
          "rust"
        ]
        "#);
    }

    #[test]
    fn test_copy_pasta() {
        let pasta = copy_pasta("rust").expect("should find rust pasta");

        insta::assert_snapshot!(pasta, @"Rust has zero-cost abstractions, move semantics, guaranteed memory safety, threads without data races, trait-based generics, pattern matching, type inference, minimal runtime and efficient C bindings.");
    }

    #[test]
    fn test_copy_pasta_nonexistent() {
        let pasta = copy_pasta("nonexistent");
        assert!(pasta.is_none());
    }
}

documentation is written in a casual style.

we don't use `unwrap` or `expect` in code, except in tests and extraordinary situations.
we avoid falling back to defaults via methods like `unwrap_or_default` or `unwrap_or_else`,
instead preferring error handling via `Result`.
we instead prefer to create bespoke error types with `thiserror` and crate-level `enums`.

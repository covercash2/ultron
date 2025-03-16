documentation is written in a casual style.

we don't use `unwrap` or `expect` in code, except in tests and extraordinary situations.
we instead prefer to create bespoke error types with `thiserror` and crate-level `enums`.

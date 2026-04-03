# This is a rust serialization framework (serialization only)

Follow general rules:
1. Always check if it compile by `cargo check`(`--release` is usually unnecessary).
2. Always write tests.
3. Always run `cargo fmt`.

# Common mistake
1. This is not a serde wrapper, NEVER follow serde's design.
2. Reasoning about ownership of `Future`, mark it explicitly in docs.
3. Async first.

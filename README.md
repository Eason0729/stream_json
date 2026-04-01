# stream-json

A **streaming, async-only** JSON serialization framework for Rust. Designed to handle strings up to 1TB without loading into memory.

## Features

- **Async-first**: No sync serialization. Uses `poll` interface for integration with `futures`.
- **Streaming**: Serializes data in chunks (128KB default) to avoid memory exhaustion.
- **Token-based**: `Token` enum for structured serialization to JSON tokens.
- **Derive macros**: `#[derive(Serialize)]` for structs and enums.
- **No heap stress**: Large strings are chunked and streamed, not buffered.

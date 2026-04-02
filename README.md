# stream-json

A streaming, async-only JSON serialization framework for Rust.

## Features

- Async-first `poll`-based serialization
- Streaming output in chunks
- Exact `size()` queries for supported values
- `#[derive(IntoSerializer)]` for structs and enums

## Size Queries

`IntoSerializer::size()` returns the exact serialized byte size when known.
Use it when you need a `Content-Length`-style value before streaming.

Examples:

```rust
use stream_json::serde::IntoSerializer;

assert_eq!("hello".size(), Some(7));
assert_eq!((42i64).size(), Some(2));
```

If a value cannot report its size, the default implementation returns `None`.

## Base64 Embed

`Base64EmbedFile::new(reader, expected_size).await` now validates the provided
size and preloads the first bytes needed for MIME detection.

```rust
use futures::io::Cursor;
use stream_json::Base64EmbedFile;

# async fn demo() -> Result<(), stream_json::Error> {
let embed = Base64EmbedFile::new(Cursor::new(vec![0u8; 16]), 16).await?;
assert!(embed.size().is_some());
# Ok(())
# }
```

## Usage

```rust
use stream_json::serde::IntoSerializer;

#[derive(IntoSerializer)]
struct Person {
    name: String,
    age: i32,
}

let person = Person {
    name: "Alice".to_string(),
    age: 30,
};

assert_eq!(person.size(), Some(25));
```

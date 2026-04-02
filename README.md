# stream-json

A **streaming, async-only** JSON serialization framework for Rust. Designed to handle strings up to 1TB without loading into memory.

## Features

- **Async-first**: No sync serialization. Uses `poll` interface for integration with `futures`.
- **Streaming**: Serializes data in chunks (128KB default) to avoid memory exhaustion.
- **Derive macros**: `#[derive(Serialize)]` for structs and enums.
- **No heap stress**: Large strings are chunked and streamed, not buffered.

## Usage

```rust
use std::task::{Context, Poll};
use bytes::Bytes;
use futures::io::Cursor;
use stream_json::serde::{Serialize, Serializer};
use stream_json::base64_embed::Base64EmbedFile;

#[derive(Serialize)]
struct Person {
    #[stream(rename = "user_name")]
    name: String,
    #[stream(skip_serialize_if = "String::is_empty")]
    nickname: String,
    avatar: Base64EmbedFile,
}

#[derive(Serialize)]
enum Event {
    #[stream(rename = "click")]
    Click { x: f64, y: f64 },
    #[stream(rename = "key_press")]
    KeyPress(String),
    #[stream(rename = "idle")]
    Idle,
}

async fn example() {
    let avatar_data = vec![0x89, 0x50, 0x4E, 0x47];
    let avatar = Base64EmbedFile::new(Cursor::new(avatar_data));

    let person = Person {
        name: "Alice".to_string(),
        nickname: "".to_string(),
        avatar,
    };

    let ser = person.into_serializer();
    let mut ser = Box::pin(ser);

    while let Poll::Ready(Some(Ok(chunk))) = ser.as_mut().poll(&mut Context::empty()) {
        println!("{:?}", chunk);
    }
}
```

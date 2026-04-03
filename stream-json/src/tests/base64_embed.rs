use bytes::Bytes;
use futures::io::Cursor;
use futures_core::task::Poll;

use crate::base64_embed::Base64EmbedFile;
use crate::error::Error;
use crate::serde::{IntoSerializer, Serializer};
use stream_json_macros::IntoSerializer;

fn poll_next<S: Serializer + Unpin>(ser: &mut S) -> Option<Result<Bytes, Error>> {
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(&waker);
    match ser.poll(&mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => panic!("poll returned Pending"),
    }
}

fn collect_bytes<S: Serializer + Unpin>(mut ser: S) -> Vec<u8> {
    let mut result = Vec::new();
    while let Some(Ok(bytes)) = poll_next(&mut ser) {
        result.extend_from_slice(&bytes);
    }
    result
}

#[test]
fn basic_base64_embed_png_magic_bytes() {
    let png_header = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52,
    ];
    let cursor = Cursor::new(png_header);
    let ser = Base64EmbedFile::new(cursor, 16, "image/png".to_string()).unwrap();
    assert_eq!(ser.size(), Some("data:image/png;base64,".len() + 24));

    let output = collect_bytes(ser);
    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.starts_with("data:image/png;base64,"));
}

#[test]
fn basic_base64_embed_with_into_serializer() {
    use crate::IntoSerializer;

    let png_header = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52,
    ];
    let cursor = Cursor::new(png_header);
    let embed = Base64EmbedFile::new(cursor, 16, "image/png".to_string()).unwrap();
    let ser = embed.into_serializer();

    let output = collect_bytes(ser);
    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.starts_with("data:image/png;base64,"));
}

#[test]
fn basic_base64_embed_empty_data() {
    let data = Vec::new();
    let cursor = Cursor::new(data);
    let ser = Base64EmbedFile::new(cursor, 0, "application/octet-stream".to_string()).unwrap();
    assert_eq!(
        ser.size(),
        Some("data:application/octet-stream;base64,".len())
    );

    let output = collect_bytes(ser);
    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.starts_with("data:application/octet-stream;base64,"));
}

#[cfg(target_os = "linux")]
mod memory_tests {
    use super::*;
    use crate::tests::memory as mem;

    fn touch_pages(bytes: usize) -> Vec<u8> {
        mem::touch_pages(bytes)
    }

    fn memory_usage() -> Option<mem::MemoryUsage> {
        mem::memory_usage()
    }

    fn assert_physical_memory_increases_by_at_least<F, T>(min_delta: usize, f: F)
    where
        F: FnOnce() -> T,
    {
        mem::assert_physical_memory_increases_by_at_least(min_delta, f)
    }

    #[test]
    fn base64_embed_memory_large_file() {
        let file_size = 10 * 1024 * 1024;
        touch_pages(file_size);

        assert_physical_memory_increases_by_at_least(0, || {
            let data = std::fs::read("testdata/large_image.png").expect("failed to read file");
            let cursor = Cursor::new(data);
            let ser =
                Base64EmbedFile::new(cursor, 10 * 1024 * 1024, "image/png".to_string()).unwrap();

            let output = collect_bytes(ser);
            std::hint::black_box(output)
        });
    }
}

#[derive(IntoSerializer)]
struct OpenAiRequest {
    model: String,
    image_data: Base64EmbedFile<Cursor<Vec<u8>>>,
}

#[test]
fn openai_vision_request_with_base64_image() {
    let png_header = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52,
    ];
    let cursor = Cursor::new(png_header);

    let request = OpenAiRequest {
        model: "gpt-4o".to_string(),
        image_data: Base64EmbedFile::new(cursor, 16, "image/png".to_string()).unwrap(),
    };

    let bytes = collect_bytes(request.into_serializer());
    let output_str = String::from_utf8(bytes).unwrap();

    assert!(output_str.starts_with(r#"{"model":"gpt-4o","image_data":"#));
    assert!(output_str.contains("data:image/png;base64,"));
}

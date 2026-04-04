use base64::Engine;
use bytes::Bytes;
use futures::io::Cursor;
use futures_core::task::Poll;

use crate::base64_embed::{Base64EmbedFile, Base64EmbedURL};
use crate::error::Error;
use crate::serde::Serializer;

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
    let ser = Base64EmbedURL::new(cursor, 16, "image/png".to_string()).unwrap();
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
    let embed = Base64EmbedURL::new(cursor, 16, "image/png".to_string()).unwrap();
    let ser = embed.into_serializer();

    let output = collect_bytes(ser);
    let output_str = String::from_utf8(output).unwrap();

    assert!(output_str.starts_with("data:image/png;base64,"));
}

#[test]
fn basic_base64_embed_empty_data() {
    let data = Vec::new();
    let cursor = Cursor::new(data);
    let ser = Base64EmbedURL::new(cursor, 0, "application/octet-stream".to_string()).unwrap();
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
                Base64EmbedURL::new(cursor, 10 * 1024 * 1024, "image/png".to_string()).unwrap();

            let output = collect_bytes(ser);
            std::hint::black_box(output)
        });
    }
}

#[test]
fn base64_embed_correct_size() {
    let data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let cursor = Cursor::new(data.clone());
    let ser = Base64EmbedURL::new(cursor, 8, "image/png".to_string()).unwrap();

    let output = collect_bytes(ser);
    let output_str = String::from_utf8(output).unwrap();

    let expected_base64 = base64::engine::general_purpose::STANDARD.encode(&data);
    assert!(output_str.contains(&expected_base64));
}

#[test]
fn base64_embed_early_eof_error() {
    let data = vec![0x89, 0x50, 0x4E, 0x47];
    let cursor = Cursor::new(data);
    let mut ser = Base64EmbedURL::new(cursor, 16, "image/png".to_string()).unwrap();

    let mut result = Vec::new();
    let mut err = None;
    while let Some(r) = poll_next(&mut ser) {
        match r {
            Ok(bytes) => result.extend_from_slice(&bytes),
            Err(e) => {
                err = Some(e);
                break;
            }
        }
    }

    assert!(err.is_some());
    let err = err.unwrap();
    assert!(err.to_string().contains("size mismatch"));
    assert!(err.to_string().contains("expected 16"));
}

#[test]
fn base64_embed_late_eof_truncates() {
    let data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0xFF, 0xFE,
    ];
    let cursor = Cursor::new(data.clone());
    let ser = Base64EmbedURL::new(cursor, 16, "image/png".to_string()).unwrap();

    let output = collect_bytes(ser);
    let output_str = String::from_utf8(output).unwrap();

    let expected_base64 = base64::engine::general_purpose::STANDARD.encode(&data[..16]);
    assert!(output_str.contains(&expected_base64));
    assert!(!output_str.contains(&base64::engine::general_purpose::STANDARD.encode(&data[16..])));
}

#[test]
fn base64_embed_file_basic() {
    let png_header = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52,
    ];
    let cursor = Cursor::new(png_header.clone());
    let ser = Base64EmbedFile::new(cursor, 16).unwrap();
    assert_eq!(ser.size(), Some(24));

    let output = collect_bytes(ser);
    let output_str = String::from_utf8(output).unwrap();

    let expected_base64 = base64::engine::general_purpose::STANDARD.encode(&png_header);
    assert_eq!(output_str, expected_base64);
}

#[test]
fn base64_embed_file_empty_data() {
    let data = Vec::new();
    let cursor = Cursor::new(data);
    let ser = Base64EmbedFile::new(cursor, 0).unwrap();
    assert_eq!(ser.size(), Some(0));

    let output = collect_bytes(ser);
    let output_str = String::from_utf8(output).unwrap();

    assert_eq!(output_str, "");
}

#[test]
fn base64_embed_file_correct_size() {
    let data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let cursor = Cursor::new(data.clone());
    let ser = Base64EmbedFile::new(cursor, 8).unwrap();

    let output = collect_bytes(ser);
    let output_str = String::from_utf8(output).unwrap();

    let expected_base64 = base64::engine::general_purpose::STANDARD.encode(&data);
    assert_eq!(output_str, expected_base64);
}

#[test]
fn base64_embed_file_early_eof_error() {
    let data = vec![0x89, 0x50, 0x4E, 0x47];
    let cursor = Cursor::new(data);
    let mut ser = Base64EmbedFile::new(cursor, 16).unwrap();

    let mut result = Vec::new();
    let mut err = None;
    while let Some(r) = poll_next(&mut ser) {
        match r {
            Ok(bytes) => result.extend_from_slice(&bytes),
            Err(e) => {
                err = Some(e);
                break;
            }
        }
    }

    assert!(err.is_some());
    let err = err.unwrap();
    assert!(err.to_string().contains("size mismatch"));
    assert!(err.to_string().contains("expected 16"));
}

#[test]
fn base64_embed_file_late_eof_truncates() {
    let data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0xFF, 0xFE,
    ];
    let cursor = Cursor::new(data.clone());
    let ser = Base64EmbedFile::new(cursor, 16).unwrap();

    let output = collect_bytes(ser);
    let output_str = String::from_utf8(output).unwrap();

    let expected_base64 = base64::engine::general_purpose::STANDARD.encode(&data[..16]);
    assert_eq!(output_str, expected_base64);
}

#[test]
fn base64_embed_file_with_into_serializer() {
    use crate::IntoSerializer;

    let png_header = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52,
    ];
    let cursor = Cursor::new(png_header.clone());
    let embed = Base64EmbedFile::new(cursor, 16).unwrap();
    let ser = embed.into_serializer();

    let output = collect_bytes(ser);
    let output_str = String::from_utf8(output).unwrap();

    let expected_base64 = base64::engine::general_purpose::STANDARD.encode(&png_header);
    assert_eq!(output_str, expected_base64);
}

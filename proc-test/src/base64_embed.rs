use futures::io::Cursor;
use stream_json::Base64EmbedURL;

use super::collect_bytes;
use stream_json::serde::IntoSerializer;

#[derive(IntoSerializer)]
pub struct OpenAiRequest {
    pub model: String,
    pub image_data: Base64EmbedURL<Cursor<Vec<u8>>>,
}

#[test]
fn test_openai_vision_request_with_base64_image() {
    let png_header = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52,
    ];
    let cursor = Cursor::new(png_header);

    let request = OpenAiRequest {
        model: "gpt-4o".to_string(),
        image_data: Base64EmbedURL::new(cursor, 16, "image/png".to_string()).unwrap(),
    };

    let bytes = collect_bytes(request.into_serializer());
    let output_str = String::from_utf8(bytes).unwrap();

    assert!(output_str.starts_with(r#"{"model":"gpt-4o","image_data":"#));
    assert!(output_str.contains(r#""data:image/png;base64,"#));
    assert!(output_str.ends_with(r#""}"#));
}

#[test]
fn test_base64_embed_url_size_matches_actual() {
    let png_header = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52,
    ];
    let actual_size = png_header.len();
    let cursor = Cursor::new(png_header.clone());

    let embed = Base64EmbedURL::new(cursor, actual_size, "image/png".to_string()).unwrap();
    let size = embed.size();

    let ser = embed.into_serializer();
    let bytes = collect_bytes(ser);
    let streamed_size = bytes.len();

    assert_eq!(
        size,
        Some(streamed_size),
        "size() should match actual streamed bytes for Base64EmbedURL"
    );
}

#[test]
fn test_base64_embed_url_serializes_as_quoted_json_string() {
    let png_header = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52,
    ];
    let cursor = Cursor::new(png_header);

    let embed = Base64EmbedURL::new(cursor, 16, "image/png".to_string()).unwrap();
    let bytes = collect_bytes(embed.into_serializer());
    let output_str = String::from_utf8(bytes).unwrap();

    assert!(output_str.starts_with('"'));
    assert!(output_str.ends_with('"'));
    assert!(output_str.contains("data:image/png;base64,"));
}

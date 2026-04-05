use base64::Engine;
use futures_core::task::Poll;

use crate::base64_embed::Base64EmbedFile;
use crate::serde::IntoSerializer;
use crate::serde::Serializer;

#[test]
fn test_json_value_null() {
    let value = serde_json::Value::Null;
    assert_eq!(value.size(), Some(4));
    let mut ser = value.into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"null"),
        other => panic!("expected ready Some Ok null, got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_json_value_bool() {
    let value = serde_json::Value::Bool(true);
    assert_eq!(value.size(), Some(4));
    let mut ser = value.into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"true"),
        other => panic!("expected ready Some Ok true, got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_json_value_number() {
    let value = serde_json::json!(42);
    assert_eq!(value.size(), Some(2));
    let mut ser = value.into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"42"),
        other => panic!("expected ready Some Ok 42, got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_json_value_string() {
    let value = serde_json::Value::String("hello".to_string());
    assert_eq!(value.size(), Some(7));
    let mut ser = value.into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"\"hello\""),
        other => panic!("expected ready Some Ok \"hello\", got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_json_value_array() {
    let value = serde_json::json!([1, 2, 3]);
    let expected = b"[1,2,3]";
    assert_eq!(value.size(), Some(expected.len()));
    let mut ser = value.into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], expected),
        other => panic!("expected ready Some Ok [1,2,3], got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_json_value_object() {
    let value = serde_json::json!({"a": 1, "b": 2});
    let expected = b"{\"a\":1,\"b\":2}";
    assert_eq!(value.size(), Some(expected.len()));
    let mut ser = value.into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], expected),
        other => panic!(
            "expected ready Some Ok {{\"a\":1,\"b\":2}}, got {:?}",
            other
        ),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_json_value_nested() {
    let value = serde_json::json!({"arr": [1, {"x": 2}], "obj": {"y": 3}});
    let ser = value.into_serializer();
    let result = super::collect_bytes(ser);
    let result_str = String::from_utf8(result).unwrap();
    assert_eq!(result_str, "{\"arr\":[1,{\"x\":2}],\"obj\":{\"y\":3}}");
}

#[test]
fn test_json_value_with_base64_data_uri() {
    let png_header = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52,
    ];
    let expected_base64 = base64::engine::general_purpose::STANDARD.encode(&png_header);
    let data_uri = format!("data:image/png;base64,{}", expected_base64);
    let value = serde_json::json!({"image": data_uri});

    let result = super::collect_bytes(value.into_serializer());
    let result_str = String::from_utf8(result).unwrap();

    assert!(result_str.starts_with("{\"image\":\"data:image/png;base64,"));
    assert!(result_str.ends_with("\"}"));
    assert!(result_str.contains(&expected_base64));
}

#[test]
fn test_json_value_base64_embed_file_to_value() {
    let png_header = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52,
    ];
    let embed = Base64EmbedFile::new(png_header.clone(), 16).unwrap();

    let mut embed_output = Vec::new();
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(&waker);
    let mut embed_ser = embed;
    while let Poll::Ready(Some(Ok(bytes))) = embed_ser.poll(&mut cx) {
        embed_output.extend_from_slice(&bytes);
    }
    let embed_str = String::from_utf8(embed_output).unwrap();

    let value = serde_json::json!({"image": embed_str});
    let result = super::collect_bytes(value.into_serializer());
    let result_str = String::from_utf8(result).unwrap();

    assert!(result_str.starts_with("{\"image\":\""));
    assert!(result_str.contains(&base64::engine::general_purpose::STANDARD.encode(&png_header)));
}

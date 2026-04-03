use crate::serde::IntoSerializer;

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

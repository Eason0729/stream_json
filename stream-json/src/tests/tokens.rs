use crate::serde::{Token, TokenSerializer};

#[test]
fn test_simple_object() {
    let tokens = [
        Token::StartObject,
        Token::Key("name"),
        Token::String("Alice"),
        Token::Comma,
        Token::Key("age"),
        Token::I64(30),
        Token::EndObject,
    ];
    let bytes = super::collect_bytes(TokenSerializer::new(&tokens));
    assert_eq!(&bytes[..], b"{\"name\":\"Alice\",\"age\":30}");
}

#[test]
fn test_nested_object() {
    let tokens = [
        Token::StartObject,
        Token::Key("person"),
        Token::StartObject,
        Token::Key("name"),
        Token::String("Bob"),
        Token::Comma,
        Token::Key("address"),
        Token::StartObject,
        Token::Key("city"),
        Token::String("NYC"),
        Token::Comma,
        Token::Key("zip"),
        Token::U64(10001),
        Token::EndObject,
        Token::EndObject,
        Token::EndObject,
    ];
    let bytes = super::collect_bytes(TokenSerializer::new(&tokens));
    assert_eq!(
        &bytes[..],
        b"{\"person\":{\"name\":\"Bob\",\"address\":{\"city\":\"NYC\",\"zip\":10001}}}"
    );
}

#[test]
fn test_simple_array() {
    let tokens = [
        Token::StartArray,
        Token::I64(1),
        Token::Comma,
        Token::I64(2),
        Token::Comma,
        Token::I64(3),
        Token::EndArray,
    ];
    let bytes = super::collect_bytes(TokenSerializer::new(&tokens));
    assert_eq!(&bytes[..], b"[1,2,3]");
}

#[test]
fn test_nested_array() {
    let tokens = [
        Token::StartArray,
        Token::StartArray,
        Token::I64(1),
        Token::Comma,
        Token::I64(2),
        Token::EndArray,
        Token::Comma,
        Token::StartArray,
        Token::I64(3),
        Token::Comma,
        Token::I64(4),
        Token::EndArray,
        Token::EndArray,
    ];
    let bytes = super::collect_bytes(TokenSerializer::new(&tokens));
    assert_eq!(&bytes[..], b"[[1,2],[3,4]]");
}

#[test]
fn test_array_of_objects() {
    let tokens = [
        Token::StartArray,
        Token::StartObject,
        Token::Key("id"),
        Token::I64(1),
        Token::EndObject,
        Token::Comma,
        Token::StartObject,
        Token::Key("id"),
        Token::I64(2),
        Token::EndObject,
        Token::EndArray,
    ];
    let bytes = super::collect_bytes(TokenSerializer::new(&tokens));
    assert_eq!(&bytes[..], b"[{\"id\":1},{\"id\":2}]");
}

#[test]
fn test_object_with_array_value() {
    let tokens = [
        Token::StartObject,
        Token::Key("name"),
        Token::String("Charlie"),
        Token::Comma,
        Token::Key("scores"),
        Token::StartArray,
        Token::F64(90.5),
        Token::Comma,
        Token::F64(85.0),
        Token::Comma,
        Token::F64(78.5),
        Token::EndArray,
        Token::EndObject,
    ];
    let bytes = super::collect_bytes(TokenSerializer::new(&tokens));
    assert_eq!(
        &bytes[..],
        b"{\"name\":\"Charlie\",\"scores\":[90.5,85,78.5]}"
    );
}

#[test]
fn test_deeply_nested_object() {
    let tokens = [
        Token::StartObject,
        Token::Key("a"),
        Token::StartObject,
        Token::Key("b"),
        Token::StartObject,
        Token::Key("c"),
        Token::StartObject,
        Token::Key("d"),
        Token::String("value"),
        Token::EndObject,
        Token::EndObject,
        Token::EndObject,
        Token::EndObject,
    ];
    let bytes = super::collect_bytes(TokenSerializer::new(&tokens));
    assert_eq!(&bytes[..], b"{\"a\":{\"b\":{\"c\":{\"d\":\"value\"}}}}");
}

#[test]
fn test_mixed_nested_structure() {
    let tokens = [
        Token::StartObject,
        Token::Key("users"),
        Token::StartArray,
        Token::StartObject,
        Token::Key("id"),
        Token::U64(1),
        Token::Comma,
        Token::Key("active"),
        Token::Bool(true),
        Token::Comma,
        Token::Key("tags"),
        Token::StartArray,
        Token::String("admin"),
        Token::Comma,
        Token::String("user"),
        Token::EndArray,
        Token::EndObject,
        Token::EndArray,
        Token::EndObject,
    ];
    let bytes = super::collect_bytes(TokenSerializer::new(&tokens));
    assert_eq!(
        &bytes[..],
        b"{\"users\":[{\"id\":1,\"active\":true,\"tags\":[\"admin\",\"user\"]}]}"
    );
}

#[test]
fn test_null_value() {
    let tokens = [Token::Null];
    let bytes = super::collect_bytes(TokenSerializer::new(&tokens));
    assert_eq!(&bytes[..], b"null");
}

#[test]
fn test_empty_object() {
    let tokens = [Token::StartObject, Token::EndObject];
    let bytes = super::collect_bytes(TokenSerializer::new(&tokens));
    assert_eq!(&bytes[..], b"{}");
}

#[test]
fn test_empty_array() {
    let tokens = [Token::StartArray, Token::EndArray];
    let bytes = super::collect_bytes(TokenSerializer::new(&tokens));
    assert_eq!(&bytes[..], b"[]");
}

use stream_json_macros::Serialize;

use crate::serde::IntoSerializer;

#[derive(Serialize)]
struct Person {
    name: String,
    age: i32,
}

#[test]
fn test_derive_named_struct() {
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
    };
    let bytes = super::collect_bytes(person.into_serializer());
    assert_eq!(&bytes[..], b"{\"name\":\"Alice\",\"age\":30}");
}

#[derive(Serialize)]
struct Point(i32, i32);

#[test]
fn test_derive_tuple_struct() {
    let point = Point(10, 20);
    let bytes = super::collect_bytes(point.into_serializer());
    assert_eq!(&bytes[..], b"{\"0\":10,\"1\":20}");
}

#[derive(Serialize)]
struct EmptyStruct {}

#[test]
fn test_derive_empty_struct() {
    let empty = EmptyStruct {};
    let bytes = super::collect_bytes(empty.into_serializer());
    assert_eq!(&bytes[..], b"{}");
}

#[derive(Serialize)]
enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn test_derive_simple_enum() {
    let color = Color::Red;
    let bytes = super::collect_bytes(color.into_serializer());
    assert_eq!(&bytes[..], b"[null]");
}

#[derive(Serialize)]
enum Status {
    Active,
    Inactive(bool),
}

#[test]
fn test_derive_enum_with_data() {
    let status = Status::Inactive(true);
    let bytes = super::collect_bytes(status.into_serializer());
    assert_eq!(&bytes[..], b"[[null]]");
}

#[derive(Serialize)]
struct PersonWithOptional {
    name: String,
    #[stream(skip_serialize_if = "|v: &String| v.is_empty()")]
    nickname: String,
    age: i32,
}

#[test]
fn test_skip_serialize_if_named_field_skipped() {
    let person = PersonWithOptional {
        name: "Alice".to_string(),
        nickname: "".to_string(),
        age: 30,
    };
    let bytes = super::collect_bytes(person.into_serializer());
    assert_eq!(&bytes[..], b"{\"name\":\"Alice\",\"age\":30}");
}

#[test]
fn test_skip_serialize_if_named_field_included() {
    let person = PersonWithOptional {
        name: "Alice".to_string(),
        nickname: "Ali".to_string(),
        age: 30,
    };
    let bytes = super::collect_bytes(person.into_serializer());
    assert_eq!(
        &bytes[..],
        b"{\"name\":\"Alice\",\"nickname\":\"Ali\",\"age\":30}"
    );
}

use super::{collect_bytes, poll_next};
use stream_json::serde::IntoSerializer;

#[derive(IntoSerializer)]
struct Person {
    name: String,
    age: i32,
}

#[derive(IntoSerializer)]
pub struct PublicPerson {
    name: String,
    age: i32,
}

pub fn public_person_serializer(person: PublicPerson) -> <PublicPerson as IntoSerializer>::S {
    person.into_serializer()
}

#[derive(IntoSerializer)]
struct RenamedPerson {
    #[stream(rename = "user_name")]
    name: String,
    #[stream(rename = "user_age")]
    age: i32,
}

#[derive(IntoSerializer)]
struct ManyFields {
    field1: String,
    field2: i32,
    field3: bool,
    field4: u64,
    field5: f64,
    field6: i8,
    field7: i16,
    field8: i32,
    field9: i64,
    field10: u8,
    field11: u16,
    field12: u32,
}

#[derive(IntoSerializer)]
struct NestedStruct {
    inner: Person,
    label: String,
}

#[derive(IntoSerializer)]
struct DeeplyNested {
    level1: NestedStruct,
    level2: NestedStruct,
}

#[test]
fn test_derive_named_struct() {
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
    };
    assert_eq!(person.size(), Some(25));
    let bytes = collect_bytes(person.into_serializer());
    assert_eq!(&bytes[..], b"{\"name\":\"Alice\",\"age\":30}");
}

#[test]
fn test_public_struct_serializer_visibility() {
    let person = PublicPerson {
        name: "Alice".to_string(),
        age: 30,
    };
    let _serializer = public_person_serializer(person);
}

#[derive(IntoSerializer)]
struct Point(i32, i32);

#[test]
fn test_derive_tuple_struct() {
    let point = Point(10, 20);
    let bytes = collect_bytes(point.into_serializer());
    assert_eq!(&bytes[..], b"[10,20]");
}

#[derive(IntoSerializer)]
struct EmptyStruct {}

#[test]
fn test_derive_empty_struct() {
    let empty = EmptyStruct {};
    let bytes = collect_bytes(empty.into_serializer());
    assert_eq!(&bytes[..], b"{}");
}

#[test]
fn test_renamed_fields() {
    let person = RenamedPerson {
        name: "Bob".to_string(),
        age: 25,
    };
    let bytes = collect_bytes(person.into_serializer());
    assert_eq!(&bytes[..], b"{\"user_name\":\"Bob\",\"user_age\":25}");
}

#[test]
fn test_many_fields() {
    let many = ManyFields {
        field1: "test".to_string(),
        field2: 42,
        field3: true,
        field4: 100,
        field5: 3.14,
        field6: 1,
        field7: 2,
        field8: 3,
        field9: 4,
        field10: 5,
        field11: 6,
        field12: 7,
    };
    let bytes = collect_bytes(many.into_serializer());
    assert_eq!(
        &bytes[..],
        b"{\"field1\":\"test\",\"field2\":42,\"field3\":true,\"field4\":100,\"field5\":3.14,\"field6\":1,\"field7\":2,\"field8\":3,\"field9\":4,\"field10\":5,\"field11\":6,\"field12\":7}"
    );
}

#[test]
fn test_nested_struct() {
    let nested = NestedStruct {
        inner: Person {
            name: "Nested".to_string(),
            age: 10,
        },
        label: "label".to_string(),
    };
    let bytes = collect_bytes(nested.into_serializer());
    assert_eq!(
        &bytes[..],
        b"{\"inner\":{\"name\":\"Nested\",\"age\":10},\"label\":\"label\"}"
    );
}

#[test]
fn test_deeply_nested_struct() {
    let nested = DeeplyNested {
        level1: NestedStruct {
            inner: Person {
                name: "L1".to_string(),
                age: 1,
            },
            label: "L1".to_string(),
        },
        level2: NestedStruct {
            inner: Person {
                name: "L2".to_string(),
                age: 2,
            },
            label: "L2".to_string(),
        },
    };
    let bytes = collect_bytes(nested.into_serializer());
    assert_eq!(
        &bytes[..],
        b"{\"level1\":{\"inner\":{\"name\":\"L1\",\"age\":1},\"label\":\"L1\"},\"level2\":{\"inner\":{\"name\":\"L2\",\"age\":2},\"label\":\"L2\"}}"
    );
}

#[derive(IntoSerializer)]
struct TupleStructMultiple(i32, String, bool, f64);

#[test]
fn test_tuple_struct_multiple_fields() {
    let tuple = TupleStructMultiple(42, "hello".to_string(), true, 2.718);
    let bytes = collect_bytes(tuple.into_serializer());
    assert_eq!(&bytes[..], b"[42,\"hello\",true,2.718]");
}

#[derive(IntoSerializer)]
struct UnitStruct;

#[test]
fn test_unit_struct() {
    let unit = UnitStruct;
    let bytes = collect_bytes(unit.into_serializer());
    assert_eq!(&bytes[..], b"{}");
}

#[derive(IntoSerializer)]
struct NewtypeStruct(String);

#[test]
fn test_newtype_struct() {
    let newtype = NewtypeStruct("wrapper".to_string());
    let bytes = collect_bytes(newtype.into_serializer());
    assert_eq!(&bytes[..], b"\"wrapper\"");
}

#[derive(IntoSerializer)]
struct SingleFieldStruct {
    value: i32,
}

#[test]
fn test_single_field_struct() {
    let s = SingleFieldStruct { value: 42 };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"value\":42}");
}

#[derive(IntoSerializer)]
struct UnitStructWrapper(String);

#[test]
fn test_unit_struct_wrapper() {
    let s = UnitStructWrapper("hello".to_string());
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"\"hello\"");
}

#[derive(IntoSerializer)]
struct UnitStructWrapperMulti(String, i32);

#[test]
fn test_unit_struct_wrapper_multi() {
    let s = UnitStructWrapperMulti("hello".to_string(), 42);
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"[\"hello\",42]");
}

#[derive(IntoSerializer)]
struct UnitStructWrapperWithRename {
    #[stream(rename = "wrapped")]
    inner: UnitStructWrapper,
}

#[test]
fn test_unit_struct_wrapper_with_rename() {
    let s = UnitStructWrapperWithRename {
        inner: UnitStructWrapper("test".to_string()),
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"wrapped\":\"test\"}");
}

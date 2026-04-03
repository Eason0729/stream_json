#![allow(unused)]

use bytes::Bytes;
use futures_core::task::Poll;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::Context;
use stream_json::serde::{IntoSerializer, Serializer};

fn poll_next<S: Serializer + Unpin>(ser: &mut S) -> Option<Result<Bytes, stream_json::Error>> {
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

#[derive(IntoSerializer)]
struct Person {
    name: String,
    age: i32,
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

#[derive(IntoSerializer)]
struct Point(i32, i32);

#[test]
fn test_derive_tuple_struct() {
    let point = Point(10, 20);
    let bytes = collect_bytes(point.into_serializer());
    assert_eq!(&bytes[..], b"{\"0\":10,\"1\":20}");
}

#[derive(IntoSerializer)]
struct EmptyStruct {}

#[test]
fn test_derive_empty_struct() {
    let empty = EmptyStruct {};
    let bytes = collect_bytes(empty.into_serializer());
    assert_eq!(&bytes[..], b"{}");
}

#[derive(IntoSerializer)]
enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn test_derive_simple_enum() {
    let color = Color::Red;
    let bytes = collect_bytes(color.into_serializer());
    assert_eq!(&bytes[..], b"[red]");
}

#[derive(IntoSerializer)]
enum Status {
    Active,
    Inactive(bool),
}

#[test]
fn test_derive_enum_with_data() {
    let status = Status::Inactive(true);
    let bytes = collect_bytes(status.into_serializer());
    assert_eq!(&bytes[..], b"[[null]]");
}

#[derive(IntoSerializer)]
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
    assert_eq!(person.size(), Some(25));
    let bytes = collect_bytes(person.into_serializer());
    assert_eq!(&bytes[..], b"{\"name\":\"Alice\",\"age\":30}");
}

#[test]
fn test_skip_serialize_if_named_field_included() {
    let person = PersonWithOptional {
        name: "Alice".to_string(),
        nickname: "Ali".to_string(),
        age: 30,
    };
    assert_eq!(person.size(), Some(42));
    let bytes = collect_bytes(person.into_serializer());
    assert_eq!(
        &bytes[..],
        b"{\"name\":\"Alice\",\"nickname\":\"Ali\",\"age\":30}"
    );
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
    assert_eq!(
        &bytes[..],
        b"{\"0\":42,\"1\":\"hello\",\"2\":true,\"3\":2.718}"
    );
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
    assert_eq!(&bytes[..], b"{\"0\":\"wrapper\"}");
}

#[derive(IntoSerializer)]
enum MixedEnum {
    Unit,
    Tuple(i32, i32),
    Named { x: i32, y: i32 },
}

#[test]
fn test_mixed_enum_unit_variant() {
    let en = MixedEnum::Unit;
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[unit]");
}

#[test]
fn test_mixed_enum_tuple_variant() {
    let en = MixedEnum::Tuple(1, 2);
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[[nullnull]]");
}

#[test]
fn test_mixed_enum_named_variant() {
    let en = MixedEnum::Named { x: 10, y: 20 };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[{\"x\":null,\"y\":null}]");
}

#[derive(IntoSerializer)]
enum RenamedEnum {
    #[stream(rename = "unit_variant")]
    Unit,
    #[stream(rename = "tuple_variant")]
    Tuple(i32),
    #[stream(rename = "named_variant")]
    Named { value: i32 },
}

#[test]
fn test_renamed_enum_unit_variant() {
    let en = RenamedEnum::Unit;
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[\"unit_variant\"]");
}

#[test]
fn test_renamed_enum_tuple_variant() {
    let en = RenamedEnum::Tuple(42);
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[\"tuple_variant\"null]]");
}

#[test]
fn test_renamed_enum_named_variant() {
    let en = RenamedEnum::Named { value: 99 };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[{\"named_variant\":null}]");
}

#[derive(IntoSerializer)]
enum ComplexEnum {
    Empty {},
    SingleUnnamed(String),
    MultipleUnnamed(String, i32, bool),
    SingleNamed {
        name: String,
    },
    MultipleNamed {
        name: String,
        age: i32,
        active: bool,
    },
}

#[test]
fn test_complex_enum_empty_variant() {
    let en = ComplexEnum::Empty {};
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[null]");
}

#[test]
fn test_complex_enum_single_unnamed() {
    let en = ComplexEnum::SingleUnnamed("test".to_string());
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[[null]]");
}

#[test]
fn test_complex_enum_multiple_unnamed() {
    let en = ComplexEnum::MultipleUnnamed("test".to_string(), 42, true);
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[[nullnullnull]]");
}

#[test]
fn test_complex_enum_single_named() {
    let en = ComplexEnum::SingleNamed {
        name: "Alice".to_string(),
    };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[{\"name\":null}]");
}

#[test]
fn test_complex_enum_multiple_named() {
    let en = ComplexEnum::MultipleNamed {
        name: "Bob".to_string(),
        age: 30,
        active: true,
    };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(
        &bytes[..],
        b"[{\"name\":null,\"age\":null,\"active\":null}]"
    );
}

#[derive(IntoSerializer)]
struct MixedStruct {
    string_field: String,
    #[stream(rename = "int_field")]
    int_field: i32,
    bool_field: bool,
    #[stream(skip_serialize_if = "|v: &i32| *v == 0")]
    optional_int: i32,
}

#[test]
fn test_mixed_struct_with_mixed_attributes() {
    let s = MixedStruct {
        string_field: "hello".to_string(),
        int_field: 42,
        bool_field: false,
        optional_int: 0,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(
        &bytes[..],
        b"{\"string_field\":\"hello\",\"int_field\":42,\"bool_field\":false,}"
    );
}

#[test]
fn test_mixed_struct_with_mixed_attributes_included() {
    let s = MixedStruct {
        string_field: "world".to_string(),
        int_field: 100,
        bool_field: true,
        optional_int: 5,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(
        &bytes[..],
        b"{\"string_field\":\"world\",\"int_field\":100,\"bool_field\":true,\"optional_int\":5}"
    );
}

#[derive(IntoSerializer)]
struct AllIntegerTypes {
    i8_field: i8,
    i16_field: i16,
    i32_field: i32,
    i64_field: i64,
    u8_field: u8,
    u16_field: u16,
    u32_field: u32,
    u64_field: u64,
}

#[test]
fn test_all_integer_types() {
    let s = AllIntegerTypes {
        i8_field: -1,
        i16_field: -2,
        i32_field: -3,
        i64_field: -4,
        u8_field: 1,
        u16_field: 2,
        u32_field: 3,
        u64_field: 4,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(
        &bytes[..],
        b"{\"i8_field\":-1,\"i16_field\":-2,\"i32_field\":-3,\"i64_field\":-4,\"u8_field\":1,\"u16_field\":2,\"u32_field\":3,\"u64_field\":4}"
    );
}

#[derive(IntoSerializer)]
struct AllFloatTypes {
    f32_field: f32,
    f64_field: f64,
}

#[test]
fn test_all_float_types() {
    let s = AllFloatTypes {
        f32_field: 1.5,
        f64_field: 2.25,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"f32_field\":1.5,\"f64_field\":2.25}");
}

#[derive(IntoSerializer)]
struct BoolFields {
    true_field: bool,
    false_field: bool,
}

#[test]
fn test_bool_fields() {
    let s = BoolFields {
        true_field: true,
        false_field: false,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"true_field\":true,\"false_field\":false}");
}

#[derive(IntoSerializer)]
struct StringFields {
    empty: String,
    with_content: String,
    with_escape: String,
}

#[test]
fn test_string_fields() {
    let s = StringFields {
        empty: String::new(),
        with_content: "hello".to_string(),
        with_escape: "line\nbreak".to_string(),
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(
        &bytes[..],
        b"{\"empty\":\"\",\"with_content\":\"hello\",\"with_escape\":\"line\\nbreak\"}"
    );
}

#[derive(IntoSerializer)]
enum EnumWithRenamedFields {
    Named {
        #[stream(rename = "renamed_field")]
        field: i32,
    },
}

#[test]
fn test_enum_with_renamed_fields() {
    let en = EnumWithRenamedFields::Named { field: 42 };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[{\"renamed_field\":null}]");
}

#[derive(IntoSerializer)]
struct TupleStructWithSkip {
    field0: String,
    #[stream(skip_serialize_if = "|v: &String| v.is_empty()")]
    field1: String,
    field2: i32,
}

#[test]
fn test_tuple_struct_with_skip_all_empty() {
    let s = TupleStructWithSkip {
        field0: "first".to_string(),
        field1: String::new(),
        field2: 99,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"field0\":\"first\",\"field2\":99}");
}

#[test]
fn test_tuple_struct_with_skip_none_empty() {
    let s = TupleStructWithSkip {
        field0: "first".to_string(),
        field1: "second".to_string(),
        field2: 99,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(
        &bytes[..],
        b"{\"field0\":\"first\",\"field1\":\"second\",\"field2\":99}"
    );
}

#[derive(IntoSerializer)]
enum ThreeElements {
    First,
    Second,
    Third,
}

#[test]
fn test_three_element_enum_first() {
    let en = ThreeElements::First;
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[first]");
}

#[test]
fn test_three_element_enum_second() {
    let en = ThreeElements::Second;
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[second]");
}

#[test]
fn test_three_element_enum_third() {
    let en = ThreeElements::Third;
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[third]");
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
enum SingleVariantEnum {
    Only { value: i32 },
}

#[test]
fn test_single_variant_enum() {
    let en = SingleVariantEnum::Only { value: 77 };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[{\"value\":null}]");
}

#[derive(IntoSerializer)]
struct VecWrapper {
    name: String,
    items: Vec<i32>,
}

#[test]
fn test_vec_wrapper_struct() {
    let s = VecWrapper {
        name: "list".to_string(),
        items: vec![1, 2, 3],
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"name\":\"list\",\"items\":[1,2,3]}");
}

#[derive(IntoSerializer)]
struct OptionWrapper {
    name: String,
    value: Option<i32>,
}

#[test]
fn test_option_wrapper_some() {
    let s = OptionWrapper {
        name: "opt".to_string(),
        value: Some(42),
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"name\":\"opt\",\"value\":42}");
}

#[test]
fn test_option_wrapper_none() {
    let s = OptionWrapper {
        name: "opt".to_string(),
        value: None,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"name\":\"opt\",\"value\":null}");
}

#[derive(IntoSerializer)]
struct BoxWrapper {
    value: Box<i32>,
}

#[test]
fn test_box_wrapper() {
    let s = BoxWrapper { value: Box::new(7) };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"value\":7}");
}

#[derive(IntoSerializer)]
struct SkipWithCondition {
    name: String,
    #[stream(skip_serialize_if = "|v: &String| v.len() < 3")]
    short_name: String,
    value: i32,
}

#[test]
fn test_skip_with_condition_short() {
    let s = SkipWithCondition {
        name: "test".to_string(),
        short_name: "ab".to_string(),
        value: 42,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"name\":\"test\",\"value\":42}");
}

#[test]
fn test_skip_with_condition_long() {
    let s = SkipWithCondition {
        name: "test".to_string(),
        short_name: "longname".to_string(),
        value: 42,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(
        &bytes[..],
        b"{\"name\":\"test\",\"short_name\":\"longname\",\"value\":42}"
    );
}

#[derive(IntoSerializer)]
struct DropCheckStruct {
    a: DropCheckA,
    b: DropCheckB,
}

struct DropCheckA {
    state: Arc<AtomicBool>,
}

struct DropCheckASerializer {
    emitted: bool,
    dropped: Arc<AtomicBool>,
}

impl Serializer for DropCheckASerializer {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, stream_json::Error>>> {
        if self.emitted {
            Poll::Ready(None)
        } else {
            self.emitted = true;
            Poll::Ready(Some(Ok(Bytes::from_static(b"1"))))
        }
    }
}

impl Drop for DropCheckASerializer {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

impl IntoSerializer for DropCheckA {
    type S = DropCheckASerializer;

    fn into_serializer(self) -> Self::S {
        Self::S {
            emitted: false,
            dropped: self.state,
        }
    }
}

struct DropCheckB {
    state: Arc<AtomicBool>,
}

struct DropCheckBSerializer {
    checked: bool,
    a_dropped: Arc<AtomicBool>,
}

impl Serializer for DropCheckBSerializer {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, stream_json::Error>>> {
        if !self.checked {
            assert!(
                self.a_dropped.load(Ordering::SeqCst),
                "first field serializer was not dropped before polling the second field"
            );
            self.checked = true;
            Poll::Ready(Some(Ok(Bytes::from_static(b"2"))))
        } else {
            Poll::Ready(None)
        }
    }
}

impl IntoSerializer for DropCheckB {
    type S = DropCheckBSerializer;

    fn into_serializer(self) -> Self::S {
        Self::S {
            checked: false,
            a_dropped: self.state,
        }
    }
}

#[test]
fn test_previous_field_serializer_dropped_before_next_field() {
    let dropped = Arc::new(AtomicBool::new(false));
    let s = DropCheckStruct {
        a: DropCheckA {
            state: dropped.clone(),
        },
        b: DropCheckB {
            state: dropped.clone(),
        },
    };
    let bytes = collect_bytes(s.into_serializer());
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(&bytes[..], b"{\"a\":1,\"b\":2}");
}

#[derive(IntoSerializer)]
struct SeparateAttrsOnDifferentFields {
    #[stream(rename = "renamed_field")]
    field_a: String,
    #[stream(skip_serialize_if = "|v: &String| v.is_empty()")]
    field_b: String,
    field_c: i32,
}

#[test]
fn test_separate_attrs_on_different_fields_renamed_skipped() {
    let s = SeparateAttrsOnDifferentFields {
        field_a: "hello".to_string(),
        field_b: "".to_string(),
        field_c: 42,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"renamed_field\":\"hello\",\"field_c\":42}");
}

#[test]
fn test_separate_attrs_on_different_fields_renamed_included() {
    let s = SeparateAttrsOnDifferentFields {
        field_a: "hello".to_string(),
        field_b: "world".to_string(),
        field_c: 42,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(
        &bytes[..],
        b"{\"renamed_field\":\"hello\",\"field_b\":\"world\",\"field_c\":42}"
    );
}

#[derive(IntoSerializer)]
struct SeparateAttrsOnSameField {
    #[stream(rename = "custom_name")]
    #[stream(skip_serialize_if = "|v: &String| v.is_empty()")]
    field: String,
}

#[test]
fn test_separate_attrs_on_same_field_skipped() {
    let s = SeparateAttrsOnSameField {
        field: "".to_string(),
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{}");
}

#[test]
fn test_separate_attrs_on_same_field_included() {
    let s = SeparateAttrsOnSameField {
        field: "visible".to_string(),
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"custom_name\":\"visible\"}");
}

#[derive(IntoSerializer)]
struct CombinedAttrSameField {
    #[stream(rename = "renamed", skip_serialize_if = "|v: &String| v.is_empty()")]
    field: String,
}

#[test]
fn test_combined_attr_same_field_skipped() {
    let s = CombinedAttrSameField {
        field: "".to_string(),
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{}");
}

#[test]
fn test_combined_attr_same_field_included() {
    let s = CombinedAttrSameField {
        field: "visible".to_string(),
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"renamed\":\"visible\"}");
}

#[derive(IntoSerializer)]
struct MixedBothAttrs {
    #[stream(rename = "renamed_a")]
    #[stream(skip_serialize_if = "|v: &String| v.is_empty()")]
    field_a: String,
    #[stream(rename = "renamed_b", skip_serialize_if = "|v: &String| v.is_empty()")]
    field_b: String,
    field_c: i32,
}

#[test]
fn test_mixed_both_attrs_all_skipped() {
    let s = MixedBothAttrs {
        field_a: "".to_string(),
        field_b: "".to_string(),
        field_c: 99,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"field_c\":99}");
}

#[test]
fn test_mixed_both_attrs_first_skipped_second_included() {
    let s = MixedBothAttrs {
        field_a: "".to_string(),
        field_b: "present".to_string(),
        field_c: 99,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"renamed_b\":\"present\",\"field_c\":99}");
}

#[test]
fn test_mixed_both_attrs_both_included() {
    let s = MixedBothAttrs {
        field_a: "visible".to_string(),
        field_b: "also_visible".to_string(),
        field_c: 99,
    };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(
        &bytes[..],
        b"{\"renamed_a\":\"visible\",\"renamed_b\":\"also_visible\",\"field_c\":99}"
    );
}

#[derive(IntoSerializer)]
struct IntFieldWithRenameAndSkip {
    #[stream(rename = "renamed_int", skip_serialize_if = "|v: &i32| *v == 0")]
    value: i32,
}

#[test]
fn test_int_rename_and_skip_zero() {
    let s = IntFieldWithRenameAndSkip { value: 0 };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{}");
}

#[test]
fn test_int_rename_and_skip_nonzero() {
    let s = IntFieldWithRenameAndSkip { value: 42 };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"renamed_int\":42}");
}

#[derive(IntoSerializer)]
enum EnumVariantWithRenameAndSkip {
    #[stream(rename = "renamed_variant", skip_serialize_if = "|_: &i32| false")]
    Named { value: i32 },
}

#[test]
fn test_enum_variant_with_rename_and_skip() {
    let en = EnumVariantWithRenameAndSkip::Named { value: 10 };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"[{\"renamed_variant\":null}]");
}

#[derive(IntoSerializer)]
struct UnitStructWrapper(String);

#[test]
fn test_unit_struct_wrapper() {
    let s = UnitStructWrapper("hello".to_string());
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"0\":\"hello\"}");
}

#[derive(IntoSerializer)]
struct UnitStructWrapperMulti(String, i32);

#[test]
fn test_unit_struct_wrapper_multi() {
    let s = UnitStructWrapperMulti("hello".to_string(), 42);
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"0\":\"hello\",\"1\":42}");
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
    assert_eq!(&bytes[..], b"{\"wrapped\":{\"0\":\"test\"}}");
}

use futures::io::Cursor;
use stream_json::Base64EmbedFile;

#[derive(IntoSerializer)]
struct OpenAiRequest {
    model: String,
    image_data: Base64EmbedFile<Cursor<Vec<u8>>>,
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
        image_data: Base64EmbedFile::new(cursor, 16, "image/png".to_string()).unwrap(),
    };

    let bytes = collect_bytes(request.into_serializer());
    let output_str = String::from_utf8(bytes).unwrap();

    assert!(output_str.starts_with(r#"{"model":"gpt-4o","image_data":"#));
    assert!(output_str.contains("data:image/png;base64,"));
}

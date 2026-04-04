use super::collect_bytes;
use stream_json::serde::IntoSerializer;

#[derive(IntoSerializer)]
pub struct PersonWithOptional {
    pub name: String,
    #[stream(skip_serialize_if = "|v: &String| v.is_empty()")]
    pub nickname: String,
    pub age: i32,
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
fn test_skip_serialize_if_struct_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<PersonWithOptional>();
}

#[derive(IntoSerializer)]
pub struct MixedStruct {
    pub string_field: String,
    #[stream(rename = "int_field")]
    pub int_field: i32,
    pub bool_field: bool,
    #[stream(skip_serialize_if = "|v: &i32| *v == 0")]
    pub optional_int: i32,
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
        b"{\"string_field\":\"hello\",\"int_field\":42,\"bool_field\":false}"
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
pub struct AllIntegerTypes {
    pub i8_field: i8,
    pub i16_field: i16,
    pub i32_field: i32,
    pub i64_field: i64,
    pub u8_field: u8,
    pub u16_field: u16,
    pub u32_field: u32,
    pub u64_field: u64,
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
pub struct AllFloatTypes {
    pub f32_field: f32,
    pub f64_field: f64,
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
pub struct BoolFields {
    pub true_field: bool,
    pub false_field: bool,
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
pub struct StringFields {
    pub empty: String,
    pub with_content: String,
    pub with_escape: String,
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
pub struct TupleStructWithSkip {
    pub field0: String,
    #[stream(skip_serialize_if = "|v: &String| v.is_empty()")]
    pub field1: String,
    pub field2: i32,
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
pub struct SeparateAttrsOnDifferentFields {
    #[stream(rename = "renamed_field")]
    pub field_a: String,
    #[stream(skip_serialize_if = "|v: &String| v.is_empty()")]
    pub field_b: String,
    pub field_c: i32,
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
pub struct SeparateAttrsOnSameField {
    #[stream(rename = "custom_name")]
    #[stream(skip_serialize_if = "|v: &String| v.is_empty()")]
    pub field: String,
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
pub struct CombinedAttrSameField {
    #[stream(rename = "renamed", skip_serialize_if = "|v: &String| v.is_empty()")]
    pub field: String,
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
pub struct MixedBothAttrs {
    #[stream(rename = "renamed_a")]
    #[stream(skip_serialize_if = "|v: &String| v.is_empty()")]
    pub field_a: String,
    #[stream(rename = "renamed_b", skip_serialize_if = "|v: &String| v.is_empty()")]
    pub field_b: String,
    pub field_c: i32,
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
pub struct IntFieldWithRenameAndSkip {
    #[stream(rename = "renamed_int", skip_serialize_if = "|v: &i32| *v == 0")]
    pub value: i32,
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
pub struct WithOptionField {
    pub name: String,
    #[stream(skip_serialize_if = "|v: &Option<String>| v.is_none()")]
    pub nickname: Option<String>,
    pub age: i32,
}

#[test]
fn test_option_field_none_skipped_size_matches_actual() {
    let person = WithOptionField {
        name: "Alice".to_string(),
        nickname: None,
        age: 30,
    };
    let size = person.size();
    let bytes = collect_bytes(person.into_serializer());
    let actual_size = bytes.len();
    assert_eq!(
        size,
        Some(actual_size),
        "size() should match actual bytes when field is skipped"
    );
    assert_eq!(&bytes[..], b"{\"name\":\"Alice\",\"age\":30}");
}

#[test]
fn test_option_field_some_included_size_matches_actual() {
    let person = WithOptionField {
        name: "Alice".to_string(),
        nickname: Some("Ali".to_string()),
        age: 30,
    };
    let size = person.size();
    let bytes = collect_bytes(person.into_serializer());
    let actual_size = bytes.len();
    assert_eq!(
        size,
        Some(actual_size),
        "size() should match actual bytes when field is included"
    );
    assert_eq!(
        &bytes[..],
        b"{\"name\":\"Alice\",\"nickname\":\"Ali\",\"age\":30}"
    );
}

#[derive(IntoSerializer)]
pub struct WithVecField {
    pub name: String,
    #[stream(skip_serialize_if = "|v: &Vec<String>| v.is_empty()")]
    pub tags: Vec<String>,
    pub value: i32,
}

#[test]
fn test_vec_field_empty_skipped_size_matches_actual() {
    let person = WithVecField {
        name: "Alice".to_string(),
        tags: vec![],
        value: 30,
    };
    let size = person.size();
    let bytes = collect_bytes(person.into_serializer());
    let actual_size = bytes.len();
    assert_eq!(
        size,
        Some(actual_size),
        "size() should match actual bytes for skipped empty Vec"
    );
    assert_eq!(&bytes[..], b"{\"name\":\"Alice\",\"value\":30}");
}

#[test]
fn test_vec_field_nonempty_size_matches_actual() {
    let person = WithVecField {
        name: "Alice".to_string(),
        tags: vec!["rust".to_string()],
        value: 30,
    };
    let size = person.size();
    let bytes = collect_bytes(person.into_serializer());
    let actual_size = bytes.len();
    assert_eq!(
        size,
        Some(actual_size),
        "size() should match actual bytes for non-empty Vec"
    );
}

#[derive(IntoSerializer)]
pub struct WithTwoVecFields {
    pub name: String,
    #[stream(skip_serialize_if = "|v: &Vec<String>| v.is_empty()")]
    pub tags: Vec<String>,
    #[stream(skip_serialize_if = "|v: &Vec<String>| v.is_empty()")]
    pub extra: Vec<String>,
    pub value: i32,
}

#[test]
fn test_two_vec_fields_both_empty_size_matches_actual() {
    let person = WithTwoVecFields {
        name: "Alice".to_string(),
        tags: vec![],
        extra: vec![],
        value: 30,
    };
    let size = person.size();
    let bytes = collect_bytes(person.into_serializer());
    let actual_size = bytes.len();
    assert_eq!(
        size,
        Some(actual_size),
        "size() should match actual bytes when both Vecs are empty"
    );
    assert_eq!(&bytes[..], b"{\"name\":\"Alice\",\"value\":30}");
}

#[test]
fn test_two_vec_fields_first_empty_second_nonempty_size_matches_actual() {
    let person = WithTwoVecFields {
        name: "Alice".to_string(),
        tags: vec![],
        extra: vec!["x".to_string()],
        value: 30,
    };
    let size = person.size();
    let bytes = collect_bytes(person.into_serializer());
    let actual_size = bytes.len();
    assert_eq!(
        size,
        Some(actual_size),
        "size() should match when first empty, second non-empty"
    );
}

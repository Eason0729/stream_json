use super::collect_bytes;
use stream_json::serde::IntoSerializer;

#[derive(IntoSerializer)]
pub enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn test_derive_simple_enum() {
    let color = Color::Red;
    let bytes = collect_bytes(color.into_serializer());
    assert_eq!(&bytes[..], b"\"red\"");
}

#[derive(IntoSerializer)]
pub enum Status {
    Active,
    Inactive(bool),
}

#[test]
fn test_derive_enum_with_data() {
    let status = Status::Inactive(true);
    let bytes = collect_bytes(status.into_serializer());
    assert_eq!(&bytes[..], b"\"inactive\"");
}

#[derive(IntoSerializer)]
pub enum MixedEnum {
    Unit,
    Tuple(i32, i32),
    Named { x: i32, y: i32 },
}

#[test]
fn test_mixed_enum_unit_variant() {
    let en = MixedEnum::Unit;
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"unit\"");
}

#[test]
fn test_mixed_enum_tuple_variant() {
    let en = MixedEnum::Tuple(1, 2);
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"tuple\"");
}

#[test]
fn test_mixed_enum_named_variant() {
    let en = MixedEnum::Named { x: 10, y: 20 };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"named\"");
}

#[derive(IntoSerializer)]
pub enum RenamedEnum {
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
    assert_eq!(&bytes[..], b"\"unit_variant\"");
}

#[test]
fn test_renamed_enum_tuple_variant() {
    let en = RenamedEnum::Tuple(42);
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"tuple_variant\"");
}

#[test]
fn test_renamed_enum_named_variant() {
    let en = RenamedEnum::Named { value: 99 };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"named_variant\"");
}

#[derive(IntoSerializer)]
pub enum ComplexEnum {
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
    assert_eq!(&bytes[..], b"\"empty\"");
}

#[test]
fn test_complex_enum_single_unnamed() {
    let en = ComplexEnum::SingleUnnamed("test".to_string());
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"single_unnamed\"");
}

#[test]
fn test_complex_enum_multiple_unnamed() {
    let en = ComplexEnum::MultipleUnnamed("test".to_string(), 42, true);
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"multiple_unnamed\"");
}

#[test]
fn test_complex_enum_single_named() {
    let en = ComplexEnum::SingleNamed {
        name: "Alice".to_string(),
    };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"single_named\"");
}

#[test]
fn test_complex_enum_multiple_named() {
    let en = ComplexEnum::MultipleNamed {
        name: "Bob".to_string(),
        age: 30,
        active: true,
    };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"multiple_named\"");
}

#[derive(IntoSerializer)]
pub enum EnumWithRenamedFields {
    Named {
        #[stream(rename = "renamed_field")]
        field: i32,
    },
}

#[test]
fn test_enum_with_renamed_fields() {
    let en = EnumWithRenamedFields::Named { field: 42 };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"named\"");
}

#[derive(IntoSerializer)]
pub enum ThreeElements {
    First,
    Second,
    Third,
}

#[test]
fn test_three_element_enum_first() {
    let en = ThreeElements::First;
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"first\"");
}

#[test]
fn test_three_element_enum_second() {
    let en = ThreeElements::Second;
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"second\"");
}

#[test]
fn test_three_element_enum_third() {
    let en = ThreeElements::Third;
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"third\"");
}

#[derive(IntoSerializer)]
pub enum SingleVariantEnum {
    Only { value: i32 },
}

#[test]
fn test_single_variant_enum() {
    let en = SingleVariantEnum::Only { value: 77 };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"only\"");
}

#[derive(IntoSerializer)]
pub enum EnumVariantWithRenameAndSkip {
    #[stream(rename = "renamed_variant", skip_serialize_if = "|_: &i32| false")]
    Named { value: i32 },
}

#[test]
fn test_enum_variant_with_rename_and_skip() {
    let en = EnumVariantWithRenameAndSkip::Named { value: 10 };
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"renamed_variant\"");
}

#[derive(IntoSerializer)]
pub enum EnumVariantWithEscapedRename {
    #[stream(rename = "key\"with\\slashes\n")]
    Value,
}

#[test]
fn test_enum_variant_with_escaped_rename_size_matches_output() {
    let en = EnumVariantWithEscapedRename::Value;
    let expected_size = en.size();
    let bytes = collect_bytes(en.into_serializer());
    assert_eq!(&bytes[..], b"\"key\\\"with\\\\slashes\\n\"");
    assert_eq!(expected_size, Some(bytes.len()));
}

use super::collect_bytes;
use stream_json::serde::IntoSerializer;

#[derive(IntoSerializer)]
pub struct VecWrapper {
    pub name: String,
    pub items: Vec<i32>,
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
pub struct OptionWrapper {
    pub name: String,
    pub value: Option<i32>,
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
pub struct BoxWrapper {
    pub value: Box<i32>,
}

#[test]
fn test_box_wrapper() {
    let s = BoxWrapper { value: Box::new(7) };
    let bytes = collect_bytes(s.into_serializer());
    assert_eq!(&bytes[..], b"{\"value\":7}");
}

#[derive(IntoSerializer)]
pub struct SkipWithCondition {
    pub name: String,
    #[stream(skip_serialize_if = "|v: &String| v.len() < 3")]
    pub short_name: String,
    pub value: i32,
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

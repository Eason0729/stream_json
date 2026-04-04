use bytes::Bytes;
use futures::io::Cursor;
use futures_core::task::Poll;
use std::task::Context;
use stream_json::base64_embed::{Base64EmbedFile, Base64EmbedURL};
use stream_json::error::Error;
use stream_json::serde::{IntoSerializer, Serializer};

fn poll_next<S: Serializer + Unpin>(ser: &mut S) -> Option<Result<Bytes, Error>> {
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

fn collect_bytes_with_errors<S: Serializer + Unpin>(ser: &mut S) -> (Vec<u8>, Option<Error>) {
    let mut result = Vec::new();
    let mut err = None;
    while let Some(r) = poll_next(ser) {
        match r {
            Ok(bytes) => result.extend_from_slice(&bytes),
            Err(e) => {
                err = Some(e);
                break;
            }
        }
    }
    (result, err)
}

fn assert_size_matches_output<S: IntoSerializer>(value: S)
where
    S::S: Unpin,
{
    let size = value.size().expect("size should be known");
    let bytes = collect_bytes(value.into_serializer());
    let actual_size = bytes.len();
    assert_eq!(
        size,
        actual_size,
        "size() returned {} but actual output was {} bytes. Output: {:?}",
        size,
        actual_size,
        String::from_utf8_lossy(&bytes)
    );
}

fn assert_valid_json<S: IntoSerializer>(value: S)
where
    S::S: Unpin,
{
    let bytes = collect_bytes(value.into_serializer());
    let json_str = String::from_utf8(bytes).expect("output should be valid UTF-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("output should be valid JSON");
    let back = serde_json::to_string(&parsed).expect("re-serialization should work");
    assert_eq!(json_str, back, "JSON should be canonical");
}

#[derive(IntoSerializer)]
struct SimpleStruct {
    name: String,
    value: i32,
}

#[test]
fn test_simple_struct_size_matches_output() {
    let s = SimpleStruct {
        name: "test".to_string(),
        value: 42,
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct NestedStructLevel1 {
    inner: SimpleStruct,
    label: String,
}

#[test]
fn test_nested_struct_size_matches_output() {
    let s = NestedStructLevel1 {
        inner: SimpleStruct {
            name: "nested".to_string(),
            value: 100,
        },
        label: "label".to_string(),
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct DeeplyNested3Levels {
    level1: NestedStructLevel1,
    level2: NestedStructLevel1,
    level3: SimpleStruct,
}

#[test]
fn test_deeply_nested_3_levels_size_matches_output() {
    let s = DeeplyNested3Levels {
        level1: NestedStructLevel1 {
            inner: SimpleStruct {
                name: "l1".to_string(),
                value: 1,
            },
            label: "L1".to_string(),
        },
        level2: NestedStructLevel1 {
            inner: SimpleStruct {
                name: "l2".to_string(),
                value: 2,
            },
            label: "L2".to_string(),
        },
        level3: SimpleStruct {
            name: "l3".to_string(),
            value: 3,
        },
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct WithRename {
    #[stream(rename = "user_name")]
    name: String,
    #[stream(rename = "user_id")]
    id: i32,
    #[stream(rename = "user_score")]
    score: f64,
}

#[test]
fn test_rename_struct_size_matches_output() {
    let s = WithRename {
        name: "Alice".to_string(),
        id: 42,
        score: 98.5,
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct WithSkipSerializeIf {
    name: String,
    #[stream(skip_serialize_if = "|v: &String| v.is_empty()")]
    nickname: String,
    age: i32,
}

#[test]
fn test_skip_serialize_if_all_fields_present_size_matches() {
    let s = WithSkipSerializeIf {
        name: "Alice".to_string(),
        nickname: "Ali".to_string(),
        age: 30,
    };
    assert_size_matches_output(s);
}

#[test]
fn test_skip_serialize_if_nickname_skipped_size_matches() {
    let s = WithSkipSerializeIf {
        name: "Bob".to_string(),
        nickname: "".to_string(),
        age: 25,
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct WithSkipAndRename {
    #[stream(rename = "display_name")]
    #[stream(skip_serialize_if = "|v: &String| v.is_empty()")]
    name: String,
    value: i32,
}

#[test]
fn test_skip_and_rename_name_present_size_matches() {
    let s = WithSkipAndRename {
        name: "Visible".to_string(),
        value: 42,
    };
    assert_size_matches_output(s);
}

#[test]
fn test_skip_and_rename_name_skipped_size_matches() {
    let s = WithSkipAndRename {
        name: "".to_string(),
        value: 42,
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct WithMultipleSkips {
    name: String,
    #[stream(skip_serialize_if = "|v: &i32| *v == 0")]
    optional_int: i32,
    #[stream(skip_serialize_if = "|v: &bool| !*v")]
    optional_bool: bool,
    value: f64,
}

#[test]
fn test_multiple_skips_all_skipped_size_matches() {
    let s = WithMultipleSkips {
        name: "test".to_string(),
        optional_int: 0,
        optional_bool: false,
        value: 3.14,
    };
    assert_size_matches_output(s);
}

#[test]
fn test_multiple_skips_all_present_size_matches() {
    let s = WithMultipleSkips {
        name: "test".to_string(),
        optional_int: 99,
        optional_bool: true,
        value: 2.71,
    };
    assert_size_matches_output(s);
}

#[test]
fn test_multiple_skips_mixed_size_matches() {
    let s = WithMultipleSkips {
        name: "test".to_string(),
        optional_int: 0,
        optional_bool: true,
        value: 1.41,
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct VecOfStructs {
    items: Vec<SimpleStruct>,
}

#[test]
fn test_vec_of_structs_size_matches() {
    let s = VecOfStructs {
        items: vec![
            SimpleStruct {
                name: "a".to_string(),
                value: 1,
            },
            SimpleStruct {
                name: "b".to_string(),
                value: 2,
            },
            SimpleStruct {
                name: "c".to_string(),
                value: 3,
            },
        ],
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct VecWithSkip {
    items: Vec<WithSkipSerializeIf>,
}

#[test]
fn test_vec_with_skip_size_matches() {
    let s = VecWithSkip {
        items: vec![
            WithSkipSerializeIf {
                name: "first".to_string(),
                nickname: "".to_string(),
                age: 10,
            },
            WithSkipSerializeIf {
                name: "second".to_string(),
                nickname: "nick".to_string(),
                age: 20,
            },
            WithSkipSerializeIf {
                name: "third".to_string(),
                nickname: "".to_string(),
                age: 30,
            },
        ],
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct VecWithRename {
    items: Vec<WithRename>,
}

#[test]
fn test_vec_with_rename_size_matches() {
    let s = VecWithRename {
        items: vec![
            WithRename {
                name: "Alice".to_string(),
                id: 1,
                score: 95.0,
            },
            WithRename {
                name: "Bob".to_string(),
                id: 2,
                score: 87.5,
            },
        ],
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct NestedWithVec {
    outer: VecWrapper,
    label: String,
}

#[derive(IntoSerializer)]
struct VecWrapper {
    name: String,
    items: Vec<i32>,
}

#[test]
fn test_nested_with_vec_size_matches() {
    let s = NestedWithVec {
        outer: VecWrapper {
            name: "list".to_string(),
            items: vec![1, 2, 3],
        },
        label: "test".to_string(),
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct WithOption {
    name: String,
    value: Option<i32>,
}

#[test]
fn test_option_some_size_matches() {
    let s = WithOption {
        name: "opt".to_string(),
        value: Some(42),
    };
    assert_size_matches_output(s);
}

#[test]
fn test_option_none_size_matches() {
    let s = WithOption {
        name: "opt".to_string(),
        value: None,
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct OptionVec {
    items: Option<Vec<String>>,
}

#[test]
fn test_option_vec_size_matches() {
    let s = OptionVec {
        items: Some(vec!["a".to_string(), "b".to_string()]),
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct NestedWithOption {
    inner: WithOption,
    tag: String,
}

#[test]
fn test_nested_option_size_matches() {
    let s1 = NestedWithOption {
        inner: WithOption {
            name: "inner".to_string(),
            value: Some(99),
        },
        tag: "has_value".to_string(),
    };
    assert_size_matches_output(s1);

    let s2 = NestedWithOption {
        inner: WithOption {
            name: "inner".to_string(),
            value: None,
        },
        tag: "no_value".to_string(),
    };
    assert_size_matches_output(s2);
}

#[derive(IntoSerializer)]
struct ComplexNestedOptionSkip {
    items: Vec<WithSkipSerializeIf>,
    optional_total: Option<i32>,
    metadata: SimpleStruct,
}

#[test]
fn test_complex_nested_option_skip_size_matches() {
    let s = ComplexNestedOptionSkip {
        items: vec![
            WithSkipSerializeIf {
                name: "a".to_string(),
                nickname: "".to_string(),
                age: 1,
            },
            WithSkipSerializeIf {
                name: "b".to_string(),
                nickname: "B".to_string(),
                age: 2,
            },
        ],
        optional_total: Some(100),
        metadata: SimpleStruct {
            name: "meta".to_string(),
            value: 42,
        },
    };
    assert_size_matches_output(s);
}

#[test]
fn test_complex_nested_option_skip_none_total_size_matches() {
    let s = ComplexNestedOptionSkip {
        items: vec![
            WithSkipSerializeIf {
                name: "a".to_string(),
                nickname: "A".to_string(),
                age: 1,
            },
            WithSkipSerializeIf {
                name: "b".to_string(),
                nickname: "".to_string(),
                age: 2,
            },
        ],
        optional_total: None,
        metadata: SimpleStruct {
            name: "meta".to_string(),
            value: 42,
        },
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct Base64Wrapper {
    data: Base64EmbedURL<Cursor<Vec<u8>>>,
}

#[test]
fn test_base64_wrapper_size_matches() {
    let data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52,
    ];
    let cursor = Cursor::new(data.clone());
    let wrapper = Base64Wrapper {
        data: Base64EmbedURL::new(cursor, 16, "image/png".to_string()).unwrap(),
    };
    assert_size_matches_output(wrapper);
}

#[derive(IntoSerializer)]
struct Base64WrapperFile {
    data: Base64EmbedFile<Cursor<Vec<u8>>>,
}

#[test]
fn test_base64_wrapper_file_size_matches() {
    let data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52,
    ];
    let cursor = Cursor::new(data.clone());
    let wrapper = Base64WrapperFile {
        data: Base64EmbedFile::new(cursor, 16).unwrap(),
    };
    assert_size_matches_output(wrapper);
}

#[derive(IntoSerializer)]
struct NestedWithBase64 {
    wrapper: Base64Wrapper,
    label: String,
}

#[test]
fn test_nested_with_base64_size_matches() {
    let data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let cursor = Cursor::new(data.clone());
    let s = NestedWithBase64 {
        wrapper: Base64Wrapper {
            data: Base64EmbedURL::new(cursor, 8, "image/png".to_string()).unwrap(),
        },
        label: "test".to_string(),
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct VecWithBase64 {
    images: Vec<Base64Wrapper>,
}

#[test]
fn test_vec_with_base64_size_matches() {
    let data1 = vec![0x89, 0x50, 0x4E, 0x47];
    let data2 = vec![0x0D, 0x0A, 0x1A, 0x0A];
    let cursor1 = Cursor::new(data1);
    let cursor2 = Cursor::new(data2);
    let s = VecWithBase64 {
        images: vec![
            Base64Wrapper {
                data: Base64EmbedURL::new(cursor1.clone(), 4, "image/png".to_string()).unwrap(),
            },
            Base64Wrapper {
                data: Base64EmbedURL::new(cursor1, 4, "image/png".to_string()).unwrap(),
            },
        ],
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct WithBase64AndSkip {
    label: String,
    #[stream(skip_serialize_if = "|v: &i32| *v == 0")]
    count: i32,
    data: Base64EmbedURL<Cursor<Vec<u8>>>,
}

#[test]
fn test_base64_with_skip_size_matches() {
    let data = vec![0x89, 0x50, 0x4E, 0x47];
    let cursor = Cursor::new(data.clone());
    let s = WithBase64AndSkip {
        label: "test".to_string(),
        count: 5,
        data: Base64EmbedURL::new(cursor, 4, "image/png".to_string()).unwrap(),
    };
    assert_size_matches_output(s);
}

#[test]
fn test_base64_with_skip_count_zero_size_matches() {
    let data = vec![0x89, 0x50, 0x4E, 0x47];
    let cursor = Cursor::new(data.clone());
    let s = WithBase64AndSkip {
        label: "test".to_string(),
        count: 0,
        data: Base64EmbedURL::new(cursor, 4, "image/png".to_string()).unwrap(),
    };
    assert_size_matches_output(s);
}

#[test]
fn test_base64_early_eof_error_detected() {
    let data = vec![0x89, 0x50, 0x4E, 0x47];
    let cursor = Cursor::new(data);
    let mut ser = Base64EmbedURL::new(cursor, 16, "image/png".to_string()).unwrap();

    let (_result, err) = collect_bytes_with_errors(&mut ser);

    assert!(err.is_some(), "Expected error for early EOF but got none");
    let err_msg = err.unwrap().to_string();
    assert!(
        err_msg.contains("size mismatch"),
        "Error message should mention 'size mismatch', got: {}",
        err_msg
    );
    assert!(
        err_msg.contains("expected 16"),
        "Error message should mention 'expected 16', got: {}",
        err_msg
    );
}

struct TwoPollSerializer {
    poll_count: usize,
}

impl TwoPollSerializer {
    fn new() -> Self {
        Self { poll_count: 0 }
    }
}

impl Serializer for TwoPollSerializer {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        self.poll_count += 1;
        match self.poll_count {
            1 => Poll::Pending,
            2 => Poll::Ready(Some(Ok(Bytes::from_static(b"\"hello\"")))),
            _ => Poll::Ready(None),
        }
    }
}

impl IntoSerializer for TwoPollSerializer {
    type S = Self;
    fn into_serializer(self) -> Self::S {
        self
    }
    fn size(&self) -> Option<usize> {
        Some(7)
    }
}

impl Unpin for TwoPollSerializer {}

fn collect_bytes_allowing_pending<S: Serializer + Unpin>(mut ser: S) -> Vec<u8> {
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(&waker);
    let mut result = Vec::new();
    loop {
        match ser.poll(&mut cx) {
            Poll::Ready(Some(Ok(bytes))) => result.extend_from_slice(&bytes),
            Poll::Ready(Some(Err(e))) => panic!("unexpected error: {:?}", e),
            Poll::Ready(None) => break,
            Poll::Pending => continue,
        }
    }
    result
}

#[test]
fn test_option_with_two_poll_inner_size_matches() {
    let opt: Option<TwoPollSerializer> = Some(TwoPollSerializer::new());
    let size = opt.size().expect("size should be known");
    let bytes = collect_bytes_allowing_pending(opt.into_serializer());
    let actual_size = bytes.len();
    assert_eq!(
        size,
        actual_size,
        "size() returned {} but actual output was {} bytes. Output: {:?}",
        size,
        actual_size,
        String::from_utf8_lossy(&bytes)
    );
}

#[test]
fn test_option_none_with_pending_inner_size() {
    let opt: Option<TwoPollSerializer> = None;
    assert_size_matches_output(opt);
}

#[derive(IntoSerializer)]
struct WithPendingOption {
    name: String,
    value: Option<TwoPollSerializer>,
}

#[test]
fn test_struct_with_option_requiring_multiple_polls_size_matches() {
    let s = WithPendingOption {
        name: "test".to_string(),
        value: Some(TwoPollSerializer::new()),
    };
    let size = s.size().expect("size should be known");
    let bytes = collect_bytes_allowing_pending(s.into_serializer());
    assert_eq!(
        size,
        bytes.len(),
        "size() returned {} but actual output was {} bytes. Output: {:?}",
        size,
        bytes.len(),
        String::from_utf8_lossy(&bytes)
    );
}

#[test]
fn test_struct_with_none_option_requiring_multiple_polls_size_matches() {
    let s = WithPendingOption {
        name: "test".to_string(),
        value: None,
    };
    assert_size_matches_output(s);
}

#[test]
fn test_base64_file_early_eof_error_detected() {
    let data = vec![0x89, 0x50, 0x4E, 0x47];
    let cursor = Cursor::new(data);
    let mut ser = Base64EmbedFile::new(cursor, 16).unwrap();

    let (_result, err) = collect_bytes_with_errors(&mut ser);

    assert!(err.is_some(), "Expected error for early EOF but got none");
    let err_msg = err.unwrap().to_string();
    assert!(
        err_msg.contains("size mismatch"),
        "Error message should mention 'size mismatch', got: {}",
        err_msg
    );
}

#[test]
fn test_nested_base64_early_eof_error_detected() {
    let data = vec![0x89, 0x50, 0x4E, 0x47];
    let cursor = Cursor::new(data);
    let wrapper = Base64Wrapper {
        data: Base64EmbedURL::new(cursor, 16, "image/png".to_string()).unwrap(),
    };
    let mut ser = wrapper.into_serializer();

    let (_result, err) = collect_bytes_with_errors(&mut ser);

    assert!(err.is_some(), "Expected error for early EOF but got none");
    let err_msg = err.unwrap().to_string();
    assert!(
        err_msg.contains("size mismatch"),
        "Error message should mention 'size mismatch', got: {}",
        err_msg
    );
}

#[test]
fn test_vec_with_base64_early_eof_error_detected() {
    let data = vec![0x89, 0x50, 0x4E, 0x47];
    let cursor = Cursor::new(data);
    let wrapper = Base64Wrapper {
        data: Base64EmbedURL::new(cursor, 16, "image/png".to_string()).unwrap(),
    };
    let s = VecWithBase64 {
        images: vec![wrapper],
    };
    let mut ser = s.into_serializer();

    let (_result, err) = collect_bytes_with_errors(&mut ser);

    assert!(err.is_some(), "Expected error for early EOF but got none");
    let err_msg = err.unwrap().to_string();
    assert!(
        err_msg.contains("size mismatch"),
        "Error message should mention 'size mismatch', got: {}",
        err_msg
    );
}

#[derive(IntoSerializer)]
struct WithJsonValue {
    name: String,
    data: serde_json::Value,
}

#[test]
fn test_json_value_in_struct_size_matches() {
    let s = WithJsonValue {
        name: "test".to_string(),
        data: serde_json::json!({"key": "value", "num": 42}),
    };
    assert_size_matches_output(s);
}

#[test]
fn test_json_value_null_in_struct_size_matches() {
    let s = WithJsonValue {
        name: "test".to_string(),
        data: serde_json::Value::Null,
    };
    assert_size_matches_output(s);
}

#[test]
fn test_json_value_array_in_struct_size_matches() {
    let s = WithJsonValue {
        name: "test".to_string(),
        data: serde_json::json!([1, 2, 3, 4, 5]),
    };
    assert_size_matches_output(s);
}

#[test]
fn test_json_value_nested_object_in_struct_size_matches() {
    let s = WithJsonValue {
        name: "test".to_string(),
        data: serde_json::json!({"outer": {"inner": {"deep": true}}}),
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct VecWithJsonValue {
    items: Vec<serde_json::Value>,
}

#[test]
fn test_vec_of_json_value_size_matches() {
    let s = VecWithJsonValue {
        items: vec![
            serde_json::json!({"a": 1}),
            serde_json::json!({"b": 2}),
            serde_json::json!({"c": 3}),
        ],
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct ComplexWithJsonValueAndSkip {
    name: String,
    #[stream(skip_serialize_if = "|v: &i32| *v == 0")]
    count: i32,
    metadata: serde_json::Value,
}

#[test]
fn test_complex_json_value_with_skip_size_matches() {
    let s = ComplexWithJsonValueAndSkip {
        name: "test".to_string(),
        count: 0,
        metadata: serde_json::json!({"tags": ["rust", "json"]}),
    };
    assert_size_matches_output(s);
}

#[test]
fn test_complex_json_value_with_skip_present_size_matches() {
    let s = ComplexWithJsonValueAndSkip {
        name: "test".to_string(),
        count: 5,
        metadata: serde_json::json!({"tags": ["rust", "json"]}),
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct DeepNestingWithSkipAndRename {
    #[stream(rename = "level1")]
    l1: Level1,
}

#[derive(IntoSerializer)]
struct Level1 {
    #[stream(rename = "level2")]
    l2: Level2,
}

#[derive(IntoSerializer)]
struct Level2 {
    #[stream(skip_serialize_if = "|v: &i32| *v == 0")]
    value: i32,
    #[stream(rename = "label")]
    name: String,
}

#[test]
fn test_deep_nesting_with_skip_and_rename_size_matches() {
    let s = DeepNestingWithSkipAndRename {
        l1: Level1 {
            l2: Level2 {
                value: 42,
                name: "deep".to_string(),
            },
        },
    };
    assert_size_matches_output(s);
}

#[test]
fn test_deep_nesting_with_skip_and_rename_value_zero_size_matches() {
    let s = DeepNestingWithSkipAndRename {
        l1: Level1 {
            l2: Level2 {
                value: 0,
                name: "deep".to_string(),
            },
        },
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct AllFieldTypes {
    string_field: String,
    #[stream(rename = "int_field")]
    int_field: i32,
    bool_field: bool,
    #[stream(skip_serialize_if = "|v: &i32| *v == 0")]
    optional_int: i32,
    vec_field: Vec<i32>,
    option_field: Option<String>,
}

#[test]
fn test_all_field_types_size_matches() {
    let s = AllFieldTypes {
        string_field: "hello".to_string(),
        int_field: 42,
        bool_field: true,
        optional_int: 0,
        vec_field: vec![1, 2, 3],
        option_field: Some("world".to_string()),
    };
    assert_size_matches_output(s);
}

#[test]
fn test_all_field_types_optional_int_present_size_matches() {
    let s = AllFieldTypes {
        string_field: "hello".to_string(),
        int_field: 42,
        bool_field: true,
        optional_int: 99,
        vec_field: vec![1, 2, 3],
        option_field: Some("world".to_string()),
    };
    assert_size_matches_output(s);
}

#[test]
fn test_all_field_types_option_none_size_matches() {
    let s = AllFieldTypes {
        string_field: "hello".to_string(),
        int_field: 42,
        bool_field: true,
        optional_int: 0,
        vec_field: vec![1, 2, 3],
        option_field: None,
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct VecOfVec {
    matrix: Vec<Vec<i32>>,
}

#[test]
fn test_vec_of_vec_size_matches() {
    let s = VecOfVec {
        matrix: vec![vec![1, 2], vec![3, 4], vec![5]],
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct EmptyVecStruct {
    items: Vec<SimpleStruct>,
}

#[test]
fn test_empty_vec_struct_size_matches() {
    let s = EmptyVecStruct { items: vec![] };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct MixedEmptyAndPopulatedVec {
    empty: Vec<i32>,
    populated: Vec<SimpleStruct>,
    another_empty: Vec<String>,
}

#[test]
fn test_mixed_empty_and_populated_vec_size_matches() {
    let s = MixedEmptyAndPopulatedVec {
        empty: vec![],
        populated: vec![SimpleStruct {
            name: "test".to_string(),
            value: 1,
        }],
        another_empty: vec![],
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct WithBoxedValue {
    name: String,
    value: Box<i32>,
}

#[test]
fn test_boxed_value_size_matches() {
    let s = WithBoxedValue {
        name: "boxed".to_string(),
        value: Box::new(42),
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct VecOfBoxedValues {
    values: Vec<Box<i32>>,
}

#[test]
fn test_vec_of_boxed_values_size_matches() {
    let s = VecOfBoxedValues {
        values: vec![Box::new(1), Box::new(2), Box::new(3)],
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct WithSpecialChars {
    #[stream(rename = "key\"with\"quotes")]
    key1: String,
    #[stream(rename = "key\\with\\backslashes")]
    key2: String,
    #[stream(rename = "key\nwith\nnewlines")]
    key3: String,
}

#[test]
fn test_rename_with_special_chars_size_matches() {
    let s = WithSpecialChars {
        key1: "value1".to_string(),
        key2: "value2".to_string(),
        key3: "value3".to_string(),
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct LargeStringInStruct {
    name: String,
    #[stream(skip_serialize_if = "|v: &String| v.len() < 1000")]
    large_content: String,
}

#[test]
fn test_large_string_skip_threshold_size_matches() {
    let s = LargeStringInStruct {
        name: "test".to_string(),
        large_content: "x".repeat(500),
    };
    assert_size_matches_output(s);
}

#[test]
fn test_large_string_above_threshold_size_matches() {
    let s = LargeStringInStruct {
        name: "test".to_string(),
        large_content: "y".repeat(1500),
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct VeryDeepNesting {
    #[stream(rename = "a")]
    a: DeepLevel1,
}

#[derive(IntoSerializer)]
struct DeepLevel1 {
    #[stream(rename = "b")]
    b: DeepLevel2,
}

#[derive(IntoSerializer)]
struct DeepLevel2 {
    #[stream(rename = "c")]
    c: DeepLevel3,
}

#[derive(IntoSerializer)]
struct DeepLevel3 {
    #[stream(rename = "d")]
    d: DeepLevel4,
}

#[derive(IntoSerializer)]
struct DeepLevel4 {
    #[stream(skip_serialize_if = "|v: &i32| *v == 0")]
    value: i32,
}

#[test]
fn test_very_deep_nesting_4_levels_size_matches() {
    let s = VeryDeepNesting {
        a: DeepLevel1 {
            b: DeepLevel2 {
                c: DeepLevel3 {
                    d: DeepLevel4 { value: 42 },
                },
            },
        },
    };
    assert_size_matches_output(s);
}

#[test]
fn test_very_deep_nesting_4_levels_skip_size_matches() {
    let s = VeryDeepNesting {
        a: DeepLevel1 {
            b: DeepLevel2 {
                c: DeepLevel3 {
                    d: DeepLevel4 { value: 0 },
                },
            },
        },
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct SkipConditionWithVec {
    name: String,
    #[stream(skip_serialize_if = "|v: &Vec<i32>| v.is_empty()")]
    items: Vec<i32>,
    value: i32,
}

#[test]
fn test_skip_condition_with_empty_vec_size_matches() {
    let s = SkipConditionWithVec {
        name: "test".to_string(),
        items: vec![],
        value: 1,
    };
    assert_size_matches_output(s);
}

#[test]
fn test_skip_condition_with_populated_vec_size_matches() {
    let s = SkipConditionWithVec {
        name: "test".to_string(),
        items: vec![1, 2, 3],
        value: 1,
    };
    assert_size_matches_output(s);
}

#[derive(IntoSerializer)]
struct Base64InNestedStruct {
    outer: Base64Outer,
}

#[derive(IntoSerializer)]
struct Base64Outer {
    label: String,
    inner: Base64Inner,
}

#[derive(IntoSerializer)]
struct Base64Inner {
    #[stream(rename = "image_data")]
    data: Base64EmbedURL<Cursor<Vec<u8>>>,
}

#[test]
fn test_deeply_nested_base64_early_eof_error_detected() {
    let data = vec![0x89, 0x50, 0x4E, 0x47];
    let cursor = Cursor::new(data);
    let s = Base64InNestedStruct {
        outer: Base64Outer {
            label: "test".to_string(),
            inner: Base64Inner {
                data: Base64EmbedURL::new(cursor, 16, "image/png".to_string()).unwrap(),
            },
        },
    };
    let mut ser = s.into_serializer();

    let (_result, err) = collect_bytes_with_errors(&mut ser);

    assert!(err.is_some(), "Expected error for early EOF but got none");
    let err_msg = err.unwrap().to_string();
    assert!(
        err_msg.contains("size mismatch"),
        "Error message should mention 'size mismatch', got: {}",
        err_msg
    );
}

#[derive(IntoSerializer)]
struct ComplexSkipRenameBase64Combo {
    #[stream(rename = "display_label")]
    #[stream(skip_serialize_if = "|v: &String| v.is_empty()")]
    label: String,
    #[stream(rename = "count")]
    #[stream(skip_serialize_if = "|v: &i32| *v <= 0")]
    count: i32,
    data: Base64EmbedURL<Cursor<Vec<u8>>>,
    metadata: serde_json::Value,
}

#[test]
fn test_complex_combo_all_present_size_matches() {
    let data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let cursor = Cursor::new(data.clone());
    let s = ComplexSkipRenameBase64Combo {
        label: "Visible".to_string(),
        count: 10,
        data: Base64EmbedURL::new(cursor, 8, "image/png".to_string()).unwrap(),
        metadata: serde_json::json!({"key": "value"}),
    };
    assert_size_matches_output(s);
}

#[test]
fn test_complex_combo_label_skipped_size_matches() {
    let data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let cursor = Cursor::new(data.clone());
    let s = ComplexSkipRenameBase64Combo {
        label: "".to_string(),
        count: 10,
        data: Base64EmbedURL::new(cursor, 8, "image/png".to_string()).unwrap(),
        metadata: serde_json::json!({"key": "value"}),
    };
    assert_size_matches_output(s);
}

#[test]
fn test_complex_combo_count_skipped_size_matches() {
    let data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let cursor = Cursor::new(data.clone());
    let s = ComplexSkipRenameBase64Combo {
        label: "test".to_string(),
        count: 0,
        data: Base64EmbedURL::new(cursor, 8, "image/png".to_string()).unwrap(),
        metadata: serde_json::json!({"key": "value"}),
    };
    assert_size_matches_output(s);
}

#[test]
fn test_complex_combo_both_skipped_size_matches() {
    let data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let cursor = Cursor::new(data.clone());
    let s = ComplexSkipRenameBase64Combo {
        label: "".to_string(),
        count: 0,
        data: Base64EmbedURL::new(cursor, 8, "image/png".to_string()).unwrap(),
        metadata: serde_json::json!({"key": "value"}),
    };
    assert_size_matches_output(s);
}

#[test]
fn test_complex_combo_early_eof_error_detected() {
    let data = vec![0x89, 0x50, 0x4E, 0x47];
    let cursor = Cursor::new(data);
    let s = ComplexSkipRenameBase64Combo {
        label: "test".to_string(),
        count: 5,
        data: Base64EmbedURL::new(cursor, 16, "image/png".to_string()).unwrap(),
        metadata: serde_json::json!({"key": "value"}),
    };
    let mut ser = s.into_serializer();

    let (_result, err) = collect_bytes_with_errors(&mut ser);

    assert!(err.is_some(), "Expected error for early EOF but got none");
    let err_msg = err.unwrap().to_string();
    assert!(
        err_msg.contains("size mismatch"),
        "Error message should mention 'size mismatch', got: {}",
        err_msg
    );
}

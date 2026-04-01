use crate::serde::IntoSerializer;

#[test]
fn test_vec_into_serializer() {
    let vec = vec![1i64, 2, 3];
    let bytes = super::collect_bytes(vec.into_serializer());
    assert_eq!(&bytes[..], b"[1,2,3]");
}

#[test]
fn test_vec_string_into_serializer() {
    let vec = vec!["a".to_string(), "b".to_string()];
    let bytes = super::collect_bytes(vec.into_serializer());
    assert_eq!(&bytes[..], b"[\"a\",\"b\"]");
}

#[test]
fn test_vec_empty() {
    let vec: Vec<i64> = vec![];
    let bytes = super::collect_bytes(vec.into_serializer());
    assert_eq!(&bytes[..], b"[]");
}

use crate::serde::IntoSerializer;

#[test]
fn test_unit_into_serializer() {
    let u: () = ();
    assert_eq!(u.size(), Some(4));
    let mut ser = u.into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"null"),
        other => panic!("expected ready Some Ok null, got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_bool_into_serializer() {
    assert_eq!(true.size(), Some(4));
    let mut ser = true.into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"true"),
        other => panic!("expected ready Some Ok true, got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_i64_into_serializer() {
    assert_eq!((42i64).size(), Some(2));
    let mut ser = (42i64).into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"42"),
        other => panic!("expected ready Some Ok 42, got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_u64_into_serializer() {
    assert_eq!((42u64).size(), Some(2));
    let mut ser = (42u64).into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"42"),
        other => panic!("expected ready Some Ok 42, got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_f64_into_serializer() {
    assert_eq!((3.14f64).size(), Some(4));
    let mut ser = (3.14f64).into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"3.14"),
        other => panic!("expected ready Some Ok 3.14, got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_string_into_serializer() {
    assert_eq!(String::from("hello").size(), Some(7));
    let mut ser = String::from("hello").into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"\"hello\""),
        other => panic!("expected ready Some Ok \"hello\", got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_str_into_serializer() {
    assert_eq!("world".size(), Some(7));
    let mut ser = "world".into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"\"world\""),
        other => panic!("expected ready Some Ok \"world\", got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_ipv4_into_serializer() {
    use std::net::Ipv4Addr;
    assert_eq!(Ipv4Addr::new(127, 0, 0, 1).size(), Some(11));
    let mut ser = Ipv4Addr::new(127, 0, 0, 1).into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"\"127.0.0.1\""),
        other => panic!("expected ready Some Ok \"127.0.0.1\", got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_duration_into_serializer() {
    use std::time::Duration;
    assert_eq!(Duration::from_secs(60).size(), Some(2));
    let mut ser = Duration::from_secs(60).into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"60"),
        other => panic!("expected ready Some Ok 60, got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_bool_false() {
    assert_eq!(false.size(), Some(5));
    let mut ser = false.into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => assert_eq!(&bytes[..], b"false"),
        other => panic!("expected ready Some Ok false, got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());
}

#[test]
fn test_multibyte_char_serialization() {
    assert_eq!("中文".size(), Some(8));
    let mut ser = "中文".into_serializer();
    match super::poll_next(&mut ser) {
        Some(Ok(bytes)) => {
            assert_eq!(bytes.len(), 8);
            assert_eq!(&bytes[..], format!("\"中文\"").as_bytes());
        }
        other => panic!("expected ready Some Ok \"中文\", got {:?}", other),
    }
    assert!(super::poll_next(&mut ser).is_none());

    assert_eq!("Hello 世界".size(), Some(14));
    let ser = "Hello 世界".into_serializer();
    let bytes = super::collect_bytes(ser);
    assert_eq!(bytes.len(), 14);
    assert_eq!(&bytes[..], format!("\"Hello 世界\"").as_bytes());

    assert_eq!("🎉".size(), Some(6));
    let ser = "🎉".into_serializer();
    let bytes = super::collect_bytes(ser);
    assert_eq!(bytes.len(), 6);
    assert_eq!(&bytes[..], format!("\"🎉\"").as_bytes());

    assert_eq!("日本語".size(), Some(11));
    let ser = "日本語".into_serializer();
    let bytes = super::collect_bytes(ser);
    assert_eq!(bytes.len(), 11);
    assert_eq!(&bytes[..], format!("\"日本語\"").as_bytes());

    assert_eq!("中文\n".size(), Some(10));
    let ser = "中文\n".into_serializer();
    let bytes = super::collect_bytes(ser);
    assert_eq!(bytes.len(), 10);
    assert_eq!(&bytes[..], format!("\"中文\\n\"").as_bytes());

    assert_eq!("你好\r\n世界".size(), Some(18));
    let ser = "你好\r\n世界".into_serializer();
    let bytes = super::collect_bytes(ser);
    assert_eq!(bytes.len(), 18);
    assert_eq!(&bytes[..], format!("\"你好\\r\\n世界\"").as_bytes());
}

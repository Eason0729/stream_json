use futures_core::task::Poll;

use crate::serde::IntoSerializer;
use crate::serde::Serializer;
use crate::CHUNK_SIZE;

#[test]
fn test_large_string_chunking() {
    let large_str = "x".repeat(CHUNK_SIZE + 100);
    let mut ser = large_str.into_serializer();
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(&waker);

    let chunk1 = match ser.poll(&mut cx) {
        Poll::Ready(Some(Ok(bytes))) => bytes,
        other => panic!("expected chunk, got {:?}", other),
    };
    assert_eq!(
        &chunk1[..],
        format!("\"{}", "x".repeat(CHUNK_SIZE)).as_bytes()
    );

    let chunk2 = match ser.poll(&mut cx) {
        Poll::Ready(Some(Ok(bytes))) => bytes,
        Poll::Ready(Some(Err(e))) => panic!("expected chunk, got error: {:?}", e),
        Poll::Ready(None) => panic!("expected second chunk, got None"),
        Poll::Pending => panic!("expected chunk, got Pending"),
    };
    assert_eq!(&chunk2[..], format!("{}\"", "x".repeat(100)).as_bytes());

    assert!(matches!(ser.poll(&mut cx), Poll::Ready(None)));
}

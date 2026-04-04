use bytes::Bytes;
use futures_core::task::Poll;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::serde::{IntoSerializer, Serializer};

struct PendingOnceSerializer {
    polled: bool,
    emitted: bool,
    output: Bytes,
}

impl PendingOnceSerializer {
    fn new(output: &'static str) -> Self {
        Self {
            polled: false,
            emitted: false,
            output: Bytes::from_static(output.as_bytes()),
        }
    }
}

struct NoSizeSerializer {
    output: Bytes,
}

impl NoSizeSerializer {
    fn new(output: &'static str) -> Self {
        Self {
            output: Bytes::from_static(output.as_bytes()),
        }
    }
}

impl Serializer for PendingOnceSerializer {
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Bytes, crate::Error>>> {
        if self.emitted {
            Poll::Ready(None)
        } else if self.polled {
            self.emitted = true;
            Poll::Ready(Some(Ok(self.output.clone())))
        } else {
            self.polled = true;
            Poll::Pending
        }
    }
}

impl IntoSerializer for PendingOnceSerializer {
    type S = Self;

    fn into_serializer(self) -> Self::S {
        self
    }

    fn size(&self) -> Option<usize> {
        Some(self.output.len())
    }
}

impl Unpin for PendingOnceSerializer {}

impl Serializer for NoSizeSerializer {
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Bytes, crate::Error>>> {
        Poll::Ready(Some(Ok(self.output.clone())))
    }
}

impl IntoSerializer for NoSizeSerializer {
    type S = Self;

    fn into_serializer(self) -> Self::S {
        self
    }

    fn size(&self) -> Option<usize> {
        None
    }
}

impl Unpin for NoSizeSerializer {}

struct ErrorSerializer;

impl Serializer for ErrorSerializer {
    fn poll(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Bytes, crate::Error>>> {
        Poll::Ready(Some(Err(crate::Error::Serialization("boom".to_string()))))
    }
}

impl IntoSerializer for ErrorSerializer {
    type S = Self;

    fn into_serializer(self) -> Self::S {
        self
    }

    fn size(&self) -> Option<usize> {
        Some(4)
    }
}

impl Unpin for ErrorSerializer {}

fn collect_bytes_allowing_pending<S: Serializer + Unpin>(mut ser: S) -> Vec<u8> {
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(&waker);
    let mut result = Vec::new();
    loop {
        match ser.poll(&mut cx) {
            Poll::Ready(Some(Ok(bytes))) => result.extend_from_slice(&bytes),
            Poll::Ready(Some(Err(err))) => panic!("unexpected error: {:?}", err),
            Poll::Ready(None) => break,
            Poll::Pending => continue,
        }
    }
    result
}

#[test]
fn test_option_serializer_none_branch_size() {
    let value: Option<i32> = None;
    assert_eq!(value.size(), Some(4));
    let bytes = super::collect_bytes(value.into_serializer());
    assert_eq!(&bytes[..], b"null");
}

#[test]
fn test_option_serializer_some_branch_size() {
    let value = Some(42i32);
    assert_eq!(value.size(), Some(2));
    let bytes = super::collect_bytes(value.into_serializer());
    assert_eq!(&bytes[..], b"42");
}

#[test]
fn test_option_serializer_pending_branch() {
    let value = Some(PendingOnceSerializer::new("ok"));
    let size = value.size().expect("size should be known");
    let bytes = collect_bytes_allowing_pending(value.into_serializer());
    assert_eq!(size, bytes.len());
    assert_eq!(&bytes[..], b"ok");
}

#[test]
fn test_option_serializer_error_branch() {
    let value = Some(ErrorSerializer);
    let mut ser = value.into_serializer();
    match super::poll_next(&mut ser) {
        Some(Err(err)) => assert!(err.to_string().contains("boom")),
        other => panic!("expected error, got {:?}", other),
    }
}

#[test]
fn test_vec_serializer_empty_branch_size() {
    let value: Vec<i32> = vec![];
    assert_eq!(value.size(), Some(2));
    let bytes = super::collect_bytes(value.into_serializer());
    assert_eq!(&bytes[..], b"[]");
}

#[test]
fn test_vec_serializer_multi_element_branch_size() {
    let value = vec![1i32, 23, 456];
    assert_eq!(value.size(), Some(10));
    let bytes = super::collect_bytes(value.into_serializer());
    assert_eq!(&bytes[..], b"[1,23,456]");
}

#[test]
fn test_vec_serializer_pending_inner_branch() {
    let value = vec![
        PendingOnceSerializer::new("a"),
        PendingOnceSerializer::new("b"),
    ];
    let size = value.size().expect("size should be known");
    let bytes = collect_bytes_allowing_pending(value.into_serializer());
    assert_eq!(size, bytes.len());
    assert_eq!(&bytes[..], b"[a,b]");
}

#[test]
fn test_dyn_array_serializer_branches() {
    let items: Vec<Box<dyn Serializer + Unpin>> = vec![
        Box::new(PendingOnceSerializer::new("x")),
        Box::new(PendingOnceSerializer::new("yy")),
    ];
    let bytes = collect_bytes_allowing_pending(crate::std_impl::DynArraySerializer::new(items));
    assert_eq!(&bytes[..], b"[x,yy]");
}

#[test]
fn test_dyn_array_serializer_empty_branch() {
    let bytes = super::collect_bytes(crate::std_impl::DynArraySerializer::new(vec![]));
    assert_eq!(&bytes[..], b"[]");
}

#[test]
fn test_dyn_object_serializer_branches() {
    let fields: Vec<(Bytes, Box<dyn Serializer + Unpin>)> = vec![
        (
            Bytes::from_static(b"\"a\":"),
            Box::new(PendingOnceSerializer::new("1")),
        ),
        (
            Bytes::from_static(b"\"b\":"),
            Box::new(PendingOnceSerializer::new("2")),
        ),
    ];
    let bytes = collect_bytes_allowing_pending(crate::std_impl::DynObjectSerializer::new(fields));
    assert_eq!(&bytes[..], b"{\"a\":1,\"b\":2}");
}

#[test]
fn test_dyn_object_serializer_empty_branch() {
    let bytes = super::collect_bytes(crate::std_impl::DynObjectSerializer::new(vec![]));
    assert_eq!(&bytes[..], b"{}");
}

#[test]
fn test_std_impl_size_branches() {
    assert_eq!(("\"".to_string()).size(), Some(4));
    assert_eq!(().size(), Some(4));
    assert_eq!(true.size(), Some(4));
    assert_eq!(false.size(), Some(5));
    assert_eq!((-8i8).size(), Some(2));
    assert_eq!((123i16).size(), Some(3));
    assert_eq!((12345i32).size(), Some(5));
    assert_eq!((123456789i64).size(), Some(9));
    assert_eq!((255u8).size(), Some(3));
    assert_eq!((65535u16).size(), Some(5));
    assert_eq!((123456789u32).size(), Some(9));
    assert_eq!((123456789u64).size(), Some(9));
    assert_eq!((3.5f32).size(), Some(3));
    assert_eq!(f64::NAN.size(), Some(4));
    assert_eq!(f64::INFINITY.size(), Some(4));
    assert_eq!(f64::NEG_INFINITY.size(), Some(4));
    assert_eq!("a\"b\\c\n".size(), Some(11));
    assert_eq!(String::from("hello").size(), Some(7));
    assert_eq!(Some(12i32).size(), Some(2));
    assert_eq!(None::<i32>.size(), Some(4));
    assert_eq!(Box::new(7i32).size(), Some(1));
    assert_eq!(vec![1i32, 2, 3].size(), Some(7));
    assert_eq!((vec![] as Vec<i32>).size(), Some(2));
    assert_eq!(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)).size(), Some(11));
    assert_eq!(IpAddr::V6(Ipv6Addr::LOCALHOST).size(), Some(5));
    assert_eq!(SocketAddr::from(([127, 0, 0, 1], 8080)).size(), Some(16));
    assert_eq!(PathBuf::from("foo/bar").size(), Some(9));
    assert_eq!(Duration::from_secs(60).size(), Some(2));
    assert_eq!(SystemTime::UNIX_EPOCH.size(), Some(3));
    assert_eq!(NoSizeSerializer::new("x").size(), None);
    assert_eq!(Some(NoSizeSerializer::new("x")).size(), None);
    assert_eq!(Box::new(NoSizeSerializer::new("x")).size(), None);
    assert_eq!(vec![NoSizeSerializer::new("x")].size(), None);
}

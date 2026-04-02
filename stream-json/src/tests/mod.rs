use bytes::Bytes;
use futures_core::task::Poll;

use crate::serde::Serializer;
use crate::Error;

pub fn poll_next<S: Serializer + Unpin>(ser: &mut S) -> Option<Result<Bytes, Error>> {
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(&waker);
    match ser.poll(&mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => panic!("poll returned Pending"),
    }
}

pub fn collect_bytes<S: Serializer + Unpin>(mut ser: S) -> Vec<u8> {
    let mut result = Vec::new();
    while let Some(Ok(bytes)) = poll_next(&mut ser) {
        result.extend_from_slice(&bytes);
    }
    result
}

#[cfg(target_os = "linux")]
pub(super) mod memory;

mod chunking;
mod derive;
mod primitives;
mod tokens;
mod vec;

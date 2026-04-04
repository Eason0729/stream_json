#![allow(unused)]

use bytes::Bytes;
use futures_core::task::Poll;
use std::task::Context;
use stream_json::serde::Serializer;

mod attributes;
mod base64_embed;
mod basic;
mod containers;
mod drop_check;
mod enums;
mod serialization_edge_cases;

pub fn collect_bytes<S: Serializer + Unpin>(mut ser: S) -> Vec<u8> {
    let mut result = Vec::new();
    while let Some(Ok(bytes)) = poll_next(&mut ser) {
        result.extend_from_slice(&bytes);
    }
    result
}

pub(crate) fn poll_next<S: Serializer + Unpin>(
    ser: &mut S,
) -> Option<Result<Bytes, stream_json::Error>> {
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(&waker);
    match ser.poll(&mut cx) {
        Poll::Ready(v) => v,
        Poll::Pending => panic!("poll returned Pending"),
    }
}

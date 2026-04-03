use bytes::Bytes;
use futures_core::task::Poll;
use std::task::Context;

use crate::error::Error;
use crate::serde::{IntoSerializer, Serializer};

pub struct JsonValueSerializer {
    output: Option<Bytes>,
}

impl JsonValueSerializer {
    pub fn new(value: serde_json::Value) -> Self {
        let output = serde_json::to_string(&value).ok().map(|s| s.into());
        Self { output }
    }
}

impl Serializer for JsonValueSerializer {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        match self.output.take() {
            Some(bytes) => Poll::Ready(Some(Ok(bytes))),
            None => Poll::Ready(None),
        }
    }
}

impl Unpin for JsonValueSerializer {}

impl IntoSerializer for serde_json::value::Value {
    type S = JsonValueSerializer;

    fn into_serializer(self) -> Self::S {
        JsonValueSerializer::new(self)
    }

    fn size(&self) -> Option<usize> {
        serde_json::to_string(self).ok().map(|s| s.len())
    }
}

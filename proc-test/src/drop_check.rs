use bytes::Bytes;
use futures_core::task::Poll;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::Context;
use stream_json::error::Error;
use stream_json::serde::{IntoSerializer, Serializer};

use super::collect_bytes;

#[derive(IntoSerializer)]
pub struct DropCheckStruct {
    pub a: DropCheckA,
    pub b: DropCheckB,
}

pub struct DropCheckA {
    pub state: Arc<AtomicBool>,
}

pub struct DropCheckASerializer {
    emitted: bool,
    dropped: Arc<AtomicBool>,
}

impl Serializer for DropCheckASerializer {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        if self.emitted {
            Poll::Ready(None)
        } else {
            self.emitted = true;
            Poll::Ready(Some(Ok(Bytes::from_static(b"1"))))
        }
    }
}

impl Drop for DropCheckASerializer {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

impl IntoSerializer for DropCheckA {
    type S = DropCheckASerializer;

    fn into_serializer(self) -> Self::S {
        Self::S {
            emitted: false,
            dropped: self.state,
        }
    }
}

pub struct DropCheckB {
    pub state: Arc<AtomicBool>,
}

pub struct DropCheckBSerializer {
    checked: bool,
    a_dropped: Arc<AtomicBool>,
}

impl Serializer for DropCheckBSerializer {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        if !self.checked {
            assert!(
                self.a_dropped.load(Ordering::SeqCst),
                "first field serializer was not dropped before polling the second field"
            );
            self.checked = true;
            Poll::Ready(Some(Ok(Bytes::from_static(b"2"))))
        } else {
            Poll::Ready(None)
        }
    }
}

impl IntoSerializer for DropCheckB {
    type S = DropCheckBSerializer;

    fn into_serializer(self) -> Self::S {
        Self::S {
            checked: false,
            a_dropped: self.state,
        }
    }
}

#[test]
fn test_previous_field_serializer_dropped_before_next_field() {
    let dropped = Arc::new(AtomicBool::new(false));
    let s = DropCheckStruct {
        a: DropCheckA {
            state: dropped.clone(),
        },
        b: DropCheckB {
            state: dropped.clone(),
        },
    };
    let bytes = collect_bytes(s.into_serializer());
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(&bytes[..], b"{\"a\":1,\"b\":2}");
}

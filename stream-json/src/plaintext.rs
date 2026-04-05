use bytes::Bytes;
use futures_core::task::Poll;
use std::task::Context;

use crate::error::Error;
use crate::serde::{escape_string, escaped_string_size, IntoSerializer, Serializer};

pub struct PlainText<T: AsRef<[u8]>> {
    value: T,
}

impl<T: AsRef<[u8]>> PlainText<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

pub struct PlainTextSerializer {
    data: String,
    emitted: bool,
}

impl PlainTextSerializer {
    pub fn new(data: String) -> Self {
        Self {
            data,
            emitted: false,
        }
    }
}

impl Serializer for PlainTextSerializer {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        if self.emitted {
            Poll::Ready(None)
        } else {
            self.emitted = true;
            let escaped = escape_string(&self.data);
            Poll::Ready(Some(Ok(format!("\"{}\"", escaped).into())))
        }
    }
}

impl Unpin for PlainTextSerializer {}

impl<T: AsRef<[u8]>> IntoSerializer for PlainText<T> {
    type S = PlainTextSerializer;

    fn into_serializer(self) -> Self::S {
        let s = String::from_utf8_lossy(self.value.as_ref());
        PlainTextSerializer::new(s.into_owned())
    }

    fn size(&self) -> Option<usize> {
        let s = String::from_utf8_lossy(self.value.as_ref());
        Some(escaped_string_size(&s) + 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{collect_bytes, poll_next};

    #[test]
    fn plaintext_ascii() {
        let pt = PlainText::new(b"hello".to_vec());
        let mut ser = pt.into_serializer();
        let result = poll_next(&mut ser);
        assert_eq!(&result.unwrap().unwrap()[..], r#""hello""#.as_bytes());
        assert!(poll_next(&mut ser).is_none());
    }

    #[test]
    fn plaintext_multibyte_utf8() {
        let pt = PlainText::new(b"\xe4\xb8\xad\xe6\x96\x87".to_vec());
        let mut ser = pt.into_serializer();
        let result = poll_next(&mut ser);
        assert_eq!(&result.unwrap().unwrap()[..], r#""中文""#.as_bytes());
        assert!(poll_next(&mut ser).is_none());
    }

    #[test]
    fn plaintext_invalid_utf8() {
        let pt = PlainText::new(b"\x80\x81".to_vec());
        let mut ser = pt.into_serializer();
        let result = poll_next(&mut ser);
        let expected = format!("\"{}\"", "\u{FFFD}\u{FFFD}");
        assert_eq!(&result.unwrap().unwrap()[..], expected.as_bytes());
        assert!(poll_next(&mut ser).is_none());
    }

    #[test]
    fn plaintext_escapes() {
        let pt = PlainText::new(b"a\"b\\c\nd".to_vec());
        let mut ser = pt.into_serializer();
        let result = poll_next(&mut ser);
        assert_eq!(&result.unwrap().unwrap()[..], r#""a\"b\\c\nd""#.as_bytes());
        assert!(poll_next(&mut ser).is_none());
    }

    #[test]
    fn plaintext_size_ascii() {
        let pt = PlainText::new(b"hello".to_vec());
        assert_eq!(pt.size(), Some(7));
    }

    #[test]
    fn plaintext_size_multibyte() {
        let pt = PlainText::new(b"\xe4\xb8\xad\xe6\x96\x87".to_vec());
        assert_eq!(pt.size(), Some(8));
    }

    #[test]
    fn plaintext_size_invalid_utf8() {
        let pt = PlainText::new(b"\x80\x81".to_vec());
        assert_eq!(pt.size(), Some(8));
    }

    #[test]
    fn plaintext_size_escapes() {
        let pt = PlainText::new(b"a\"b\\c\nd".to_vec());
        assert_eq!(pt.size(), Some(12));
    }

    #[test]
    fn plaintext_size_matches_actual() {
        fn check<T: AsRef<[u8]>>(value: T) {
            let pt = PlainText::new(&value);
            let expected_size = pt.size();
            drop(pt);
            let pt = PlainText::new(value);
            let ser = pt.into_serializer();
            let bytes = collect_bytes(ser);
            assert_eq!(expected_size, Some(bytes.len()));
        }

        check(b"hello".to_vec());
        check(b"\xe4\xb8\xad\xe6\x96\x87".to_vec());
        check(b"\x80\x81".to_vec());
        check(b"a\"b\\c\nd".to_vec());
    }
}

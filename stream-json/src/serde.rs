//! # Serialization Traits and Token Types
//!
//! This module defines the core traits and types for streaming JSON serialization.
//!
//! ## Core Traits
//!
//! - [`Serializer`]: The fundamental trait for producing JSON bytes via `poll`.
//! - [`IntoSerializer`]: A conversion trait for types that can become serializers.
//! - [`IntoStreamSerializer`]: A stream wrapper around any serializer.
//!
//! ## Token Types
//!
//! - [`Token`]: An enum representing JSON tokens for structured serialization.
//! - [`TokenSerializer`]: Converts a slice of tokens into JSON bytes.
//!
//! ## Implementing Serializer
//!
//! ```
//! use bytes::Bytes;
//! use futures_core::task::Poll;
//! use std::task::Context;
//! use stream_json::serde::Serializer;
//! use stream_json::error::Error;
//!
//! pub struct MySerializer {
//!     value: String,
//!     emitted: bool,
//! }
//!
//! impl MySerializer {
//!     pub fn new(value: String) -> Self {
//!         Self { value, emitted: false }
//!     }
//! }
//!
//! impl Serializer for MySerializer {
//!     fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
//!         if self.emitted {
//!             Poll::Ready(None)
//!         } else {
//!             self.emitted = true;
//!             Poll::Ready(Some(Ok(format!("\"{}\"", self.value).into())))
//!         }
//!     }
//! }
//! ```

use bytes::Bytes;
use futures_core::stream::Stream;
use futures_core::task::Poll;
use std::task::Context;

use crate::error::Error;

/// JSON token types for structured serialization.
///
/// Use this enum to represent the structure of a JSON document as a sequence
/// of tokens. The [`TokenSerializer`] converts these tokens into bytes.
///
/// # Example
///
/// ```
/// use stream_json::serde::{Token, TokenSerializer};
///
/// let tokens = [
///     Token::StartObject,
///     Token::Key("name"),
///     Token::String("Alice"),
///     Token::Comma,
///     Token::Key("age"),
///     Token::I64(30),
///     Token::EndObject,
/// ];
///
/// let serializer = TokenSerializer::new(&tokens);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token<'a> {
    /// JSON `null` value.
    Null,
    /// JSON boolean value.
    Bool(bool),
    /// JSON signed 64-bit integer.
    I64(i64),
    /// JSON unsigned 64-bit integer.
    U64(u64),
    /// JSON 64-bit floating point number. NaN and Infinity are serialized as `null`.
    F64(f64),
    /// JSON string value (borrowed).
    String(&'a str),
    /// Array start token (`[`).
    StartArray,
    /// Array end token (`]`).
    EndArray,
    /// Object start token (`{`).
    StartObject,
    /// Object end token (`}`).
    EndObject,
    /// Object key token (emits `"key":`).
    Key(&'a str),
    /// Comma token (`,`).
    Comma,
    /// Colon token (`:`).
    Colon,
}

/// The core serializer trait for streaming JSON output.
///
/// Implementors produce JSON bytes via the `poll` method, which follows the
/// pattern of `Future::poll`. The serializer yields `Some(Ok(Bytes))` chunks
/// until it is done, then returns `None`.
///
/// # Using with IntoStreamSerializer
///
/// ```
/// use stream_json::serde::{Serializer, IntoSerializer};
/// use stream_json::IntoStreamSerializer;
/// use futures_core::stream::Stream;
///
/// let serializer = vec![1, 2, 3].into_serializer();
/// let mut stream = IntoStreamSerializer::new(serializer);
///
/// // Use poll_next directly (no async)
/// let waker = std::task::Waker::noop();
/// let mut cx = std::task::Context::from_waker(&waker);
/// let poll = std::pin::Pin::new(&mut stream).poll_next(&mut cx);
/// ```
pub trait Serializer {
    /// Produces the next chunk of JSON bytes.
    ///
    /// Returns:
    /// - `Poll::Ready(Some(Ok(bytes)))` when a chunk is available
    /// - `Poll::Ready(None)` when serialization is complete
    /// - `Poll::Pending` when more data is not yet available
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>>;
}

/// A type that can be converted into a [`Serializer`].
///
/// This trait is implemented for all primitive types (`bool`, `i64`, `u64`,
/// `f64`, `String`, `&str`) and `Vec<T>` where `T: IntoSerializer`.
/// It also exposes [`IntoSerializer::size`], which returns the exact serialized
/// byte size when known.
///
/// # Example
///
/// ```
/// use stream_json::serde::IntoSerializer;
///
/// // i64 -> I64Serializer
/// let ser = 42i64.into_serializer();
///
/// // String -> StringSerializer
/// let ser = "hello".to_string().into_serializer();
///
/// // Vec<i32> -> VecSerializer<i32>
/// let ser = vec![1, 2, 3].into_serializer();
/// ```
pub trait IntoSerializer {
    /// The serializer type produced by this conversion.
    type S: Serializer + Unpin;

    /// Converts this value into a serializer.
    fn into_serializer(self) -> Self::S;

    /// Returns the exact serialized byte size when known.
    ///
    /// Returns `None` if unknown.
    fn size(&self) -> Option<usize> {
        None
    }

    /// Converts this value into a stream of bytes.
    ///
    /// This is a convenience method that wraps the serializer in
    /// [`IntoStreamSerializer`].
    ///
    /// ```
    /// use stream_json::serde::IntoSerializer;
    /// use stream_json::IntoStreamSerializer;
    ///
    /// let vec: Vec<i32> = vec![1, 2, 3];
    /// // Vec<i32> implements IntoSerializer
    /// let stream = vec.into_stream(); // Returns IntoStreamSerializer<VecSerializer<i32>>
    /// ```
    fn into_stream(self) -> IntoStreamSerializer<Self::S>
    where
        Self: Sized,
    {
        IntoStreamSerializer::new(self.into_serializer())
    }
}

/// A stream adapter that wraps a serializer.
///
/// This struct implements [`Stream`](futures_core::stream::Stream) by delegating
/// to the underlying serializer's `poll` method.
///
/// # Example
///
/// ```
/// use stream_json::serde::{IntoSerializer, IntoStreamSerializer};
/// use futures_core::stream::Stream;
///
/// let ser = vec!["a", "b"].into_serializer();
/// let mut stream = IntoStreamSerializer::new(ser);
///
/// // Poll-based iteration
/// let waker = std::task::Waker::noop();
/// let mut cx = std::task::Context::from_waker(&waker);
/// while let std::task::Poll::Ready(Some(result)) =
///     std::pin::Pin::new(&mut stream).poll_next(&mut cx)
/// {
///     println!("{:?}", result);
/// }
/// ```
#[derive(Debug)]
pub struct IntoStreamSerializer<S: Serializer + Unpin> {
    serializer: S,
    done: bool,
}

impl<S: Serializer + Unpin> IntoStreamSerializer<S> {
    /// Creates a new stream serializer from a serializer.
    pub fn new(serializer: S) -> Self {
        Self {
            serializer,
            done: false,
        }
    }
}

impl<S: Serializer + Unpin> Unpin for IntoStreamSerializer<S> {}

impl<S: Serializer + Unpin> Stream for IntoStreamSerializer<S> {
    type Item = Result<Bytes, Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }
        match self.serializer.poll(cx) {
            Poll::Ready(Some(result)) => Poll::Ready(Some(result)),
            Poll::Ready(None) => {
                self.done = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Serializer that converts a slice of [`Token`]s into JSON bytes.
///
/// This is useful for building JSON documents programmatically from tokens.
///
/// # Example
///
/// ```
/// use stream_json::serde::{Token, TokenSerializer};
///
/// let tokens = [
///     Token::StartArray,
///     Token::I64(1),
///     Token::Comma,
///     Token::I64(2),
///     Token::Comma,
///     Token::I64(3),
///     Token::EndArray,
/// ];
///
/// let serializer = TokenSerializer::new(&tokens);
/// ```
pub struct TokenSerializer<'a> {
    tokens: &'a [Token<'a>],
    pos: usize,
}

impl<'a> TokenSerializer<'a> {
    /// Creates a new token serializer from a slice of tokens.
    pub fn new(tokens: &'a [Token<'a>]) -> Self {
        Self { tokens, pos: 0 }
    }
}

impl<'a> Serializer for TokenSerializer<'a> {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        if self.pos >= self.tokens.len() {
            return Poll::Ready(None);
        }
        let token = &self.tokens[self.pos];
        self.pos += 1;
        let output: Bytes = match token {
            Token::Null => "null".into(),
            Token::Bool(b) => b.to_string().into(),
            Token::I64(n) => n.to_string().into(),
            Token::U64(n) => n.to_string().into(),
            Token::F64(n) => n.to_string().into(),
            Token::String(s) => format!("\"{}\"", escape_string(s)).into(),
            Token::StartArray => "[".into(),
            Token::EndArray => "]".into(),
            Token::StartObject => "{".into(),
            Token::EndObject => "}".into(),
            Token::Key(k) => format!("\"{}\":", escape_string(k)).into(),
            Token::Comma => ",".into(),
            Token::Colon => ":".into(),
        };
        Poll::Ready(Some(Ok(output)))
    }
}

impl<'a> Unpin for TokenSerializer<'a> {}

impl<'a> IntoSerializer for &'a [Token<'a>] {
    type S = TokenSerializer<'a>;
    fn into_serializer(self) -> Self::S {
        TokenSerializer::new(self)
    }

    fn size(&self) -> Option<usize> {
        Some(self.iter().map(token_size).sum())
    }
}

/// Escapes a string for JSON serialization.
pub(crate) fn escape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

/// Returns the exact number of bytes required to escape a JSON string.
pub(crate) fn escaped_string_size(s: &str) -> usize {
    s.chars()
        .map(|c| match c {
            '"' | '\\' | '\n' | '\r' | '\t' => 2,
            c if c.is_control() => 6,
            c => c.len_utf8(),
        })
        .sum()
}

fn token_size(token: &Token<'_>) -> usize {
    match token {
        Token::Null => 4,
        Token::Bool(true) => 4,
        Token::Bool(false) => 5,
        Token::I64(n) => n.to_string().len(),
        Token::U64(n) => n.to_string().len(),
        Token::F64(n) => {
            if n.is_nan() || n.is_infinite() {
                4
            } else {
                n.to_string().len()
            }
        }
        Token::String(s) => escaped_string_size(s) + 2,
        Token::StartArray
        | Token::EndArray
        | Token::StartObject
        | Token::EndObject
        | Token::Comma
        | Token::Colon => 1,
        Token::Key(k) => escaped_string_size(k) + 3,
    }
}

/// State wrapper for struct fields during serialization.
///
/// This enum tracks whether a field value is still waiting, already converted
/// to a serializer, or skipped by a predicate.
pub enum FieldState<F: IntoSerializer> {
    Waiting {
        value: Option<F>,
        skip_if: Option<Box<dyn Fn(&F) -> bool>>,
    },
    Active(<F as IntoSerializer>::S),
    Skipped,
    Dropped,
}

impl<F: IntoSerializer + Unpin> Serializer for FieldState<F> {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        if !self.prepare() {
            return Poll::Ready(None);
        }
        match self {
            FieldState::Active(s) => {
                let poll = s.poll(cx);
                if matches!(poll, Poll::Ready(None)) {
                    *self = FieldState::Dropped;
                }
                poll
            }
            FieldState::Waiting { .. } | FieldState::Skipped | FieldState::Dropped => {
                Poll::Ready(None)
            }
        }
    }
}

impl<F: IntoSerializer> FieldState<F> {
    pub fn prepare(&mut self) -> bool {
        match self {
            FieldState::Waiting { value, skip_if } => {
                let Some(v) = value.take() else {
                    *self = FieldState::Skipped;
                    return false;
                };
                if skip_if.as_ref().is_some_and(|pred| pred(&v)) {
                    *self = FieldState::Skipped;
                    return false;
                }
                *self = FieldState::Active(v.into_serializer());
                true
            }
            FieldState::Active(_) => true,
            FieldState::Skipped | FieldState::Dropped => false,
        }
    }
}

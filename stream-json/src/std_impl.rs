//! # Standard Library Serializers
//!
//! This module contains serializers for primitive types and standard library
//! types. All types implement [`IntoSerializer`] and can be created directly
//! or via the trait.
//!
//! ## Value Serializers
//!
//! These serializers handle primitive values and emit them as single chunks:
//!
//! - [`UnitSerializer`] - `()` → `null`
//! - [`BoolSerializerState`] - `bool` → `true`/`false`
//! - [`I64Serializer`] - `i64` and `i8`-`i32` → JSON number
//! - [`U64Serializer`] - `u64` and `u8`-`u32` → JSON number
//! - [`F64Serializer`] - `f64` and `f32` → JSON number
//!
//! ## String Serializer
//!
//! [`StringSerializer`] handles strings with automatic chunking for large
//! content (strings larger than [`CHUNK_SIZE`](crate::CHUNK_SIZE) are split
//! into multiple 128KB chunks).
//!
//! ## Collection Serializer
//!
//! [`VecSerializer<T>`] serializes `Vec<T>` as a JSON array, properly handling
//! commas between elements.
//!
//! ## Box Serializer
//!
//! `Box<dyn Serializer + Unpin>` implements `Serializer` via delegation.
//!
//! ## Additional Types
//!
//! `IntoSerializer` is implemented for:
//! - [`std::net::IpAddr`](https://doc.rust-lang.org/std/net/enum.IpAddr.html)
//! - [`std::net::SocketAddr`](https://doc.rust-lang.org/std/net/enum.SocketAddr.html)
//! - [`std::net::Ipv4Addr`](https://doc.rust-lang.org/std/net/struct.Ipv4Addr.html)
//! - [`std::net::Ipv6Addr`](https://doc.rust-lang.org/std/net/struct.Ipv6Addr.html)
//! - [`std::path::PathBuf`](https://doc.rust-lang.org/std/path/struct.PathBuf.html)
//! - [`std::time::Duration`](https://doc.rust-lang.org/std/time/struct.Duration.html)
//! - [`std::time::SystemTime`](https://doc.rust-lang.org/std/time/struct.SystemTime.html)

use bytes::Bytes;
use futures_core::task::Poll;
use std::task::Context;

use crate::error::Error;
use crate::serde::{escape_string, IntoSerializer, Serializer};
use crate::CHUNK_SIZE;

macro_rules! impl_into_serializer_cast {
    ($ty:ty, $serializer:ty, $cast:expr) => {
        impl IntoSerializer for $ty {
            type S = $serializer;

            fn into_serializer(self) -> Self::S {
                <$serializer>::new($cast(self))
            }

            fn size(&self) -> Option<usize> {
                Some(($cast(*self)).to_string().len())
            }
        }
    };
}

/// Serializer for the unit type `()`.
///
/// Emits `null` as a single chunk.
///
/// # Example
///
/// ```
/// use stream_json::serializers::UnitSerializer;
///
/// let ser = UnitSerializer::new();
/// ```
pub struct UnitSerializer {
    emitted: bool,
}

impl UnitSerializer {
    /// Creates a new unit serializer.
    pub fn new() -> Self {
        Self { emitted: false }
    }
}

impl Default for UnitSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializer for UnitSerializer {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        if self.emitted {
            Poll::Ready(None)
        } else {
            self.emitted = true;
            Poll::Ready(Some(Ok("null".into())))
        }
    }
}

impl Unpin for UnitSerializer {}

impl IntoSerializer for () {
    type S = UnitSerializer;
    fn into_serializer(self) -> Self::S {
        UnitSerializer::new()
    }

    fn size(&self) -> Option<usize> {
        Some(4)
    }
}

/// Serializer for boolean values.
///
/// Emits `true` or `false` as a single chunk.
pub struct BoolSerializerState {
    value: bool,
    emitted: bool,
}

impl BoolSerializerState {
    /// Creates a new boolean serializer.
    pub fn new(value: bool) -> Self {
        Self {
            value,
            emitted: false,
        }
    }
}

impl Serializer for BoolSerializerState {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        if self.emitted {
            Poll::Ready(None)
        } else {
            self.emitted = true;
            let result = if self.value { "true" } else { "false" };
            Poll::Ready(Some(Ok(result.into())))
        }
    }
}

impl Unpin for BoolSerializerState {}

impl IntoSerializer for bool {
    type S = BoolSerializerState;
    fn into_serializer(self) -> Self::S {
        BoolSerializerState::new(self)
    }

    fn size(&self) -> Option<usize> {
        Some(if *self { 4 } else { 5 })
    }
}

/// Serializer for signed 64-bit integers.
///
/// Emits the integer as a JSON number. Also handles `i8`, `i16`, `i32` via
/// automatic conversion.
pub struct I64Serializer {
    value: i64,
    emitted: bool,
}

impl I64Serializer {
    /// Creates a new i64 serializer.
    pub fn new(value: i64) -> Self {
        Self {
            value,
            emitted: false,
        }
    }
}

impl Serializer for I64Serializer {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        if self.emitted {
            Poll::Ready(None)
        } else {
            self.emitted = true;
            Poll::Ready(Some(Ok(self.value.to_string().into())))
        }
    }
}

impl Unpin for I64Serializer {}

impl IntoSerializer for i64 {
    type S = I64Serializer;
    fn into_serializer(self) -> Self::S {
        I64Serializer::new(self)
    }

    fn size(&self) -> Option<usize> {
        Some(self.to_string().len())
    }
}

impl_into_serializer_cast!(i8, I64Serializer, |value: i8| value as i64);
impl_into_serializer_cast!(i16, I64Serializer, |value: i16| value as i64);
impl_into_serializer_cast!(i32, I64Serializer, |value: i32| value as i64);

/// Serializer for unsigned 64-bit integers.
///
/// Emits the integer as a JSON number. Also handles `u8`, `u16`, `u32` via
/// automatic conversion.
pub struct U64Serializer {
    value: u64,
    emitted: bool,
}

impl U64Serializer {
    /// Creates a new u64 serializer.
    pub fn new(value: u64) -> Self {
        Self {
            value,
            emitted: false,
        }
    }
}

impl Serializer for U64Serializer {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        if self.emitted {
            Poll::Ready(None)
        } else {
            self.emitted = true;
            Poll::Ready(Some(Ok(self.value.to_string().into())))
        }
    }
}

impl Unpin for U64Serializer {}

impl IntoSerializer for u64 {
    type S = U64Serializer;
    fn into_serializer(self) -> Self::S {
        U64Serializer::new(self)
    }

    fn size(&self) -> Option<usize> {
        Some(self.to_string().len())
    }
}

impl_into_serializer_cast!(u8, U64Serializer, |value: u8| value as u64);
impl_into_serializer_cast!(u16, U64Serializer, |value: u16| value as u64);
impl_into_serializer_cast!(u32, U64Serializer, |value: u32| value as u64);

/// Serializer for 64-bit floating point numbers.
///
/// NaN and Infinity are serialized as `null`. Also handles `f32` via automatic
/// conversion.
pub struct F64Serializer {
    value: f64,
    emitted: bool,
}

impl F64Serializer {
    /// Creates a new f64 serializer.
    pub fn new(value: f64) -> Self {
        Self {
            value,
            emitted: false,
        }
    }
}

impl Serializer for F64Serializer {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        if self.emitted {
            Poll::Ready(None)
        } else {
            self.emitted = true;
            let result = if self.value.is_nan() || self.value.is_infinite() {
                "null".to_string()
            } else {
                self.value.to_string()
            };
            Poll::Ready(Some(Ok(result.into())))
        }
    }
}

impl Unpin for F64Serializer {}

impl IntoSerializer for f64 {
    type S = F64Serializer;
    fn into_serializer(self) -> Self::S {
        F64Serializer::new(self)
    }

    fn size(&self) -> Option<usize> {
        Some(if self.is_nan() || self.is_infinite() {
            4
        } else {
            self.to_string().len()
        })
    }
}

impl_into_serializer_cast!(f32, F64Serializer, |value: f32| value as f64);

impl IntoSerializer for std::net::IpAddr {
    type S = StringSerializer;
    fn into_serializer(self) -> Self::S {
        StringSerializer::new(self.to_string())
    }

    fn size(&self) -> Option<usize> {
        Some(crate::serde::escaped_string_size(&self.to_string()) + 2)
    }
}

impl IntoSerializer for std::net::SocketAddr {
    type S = StringSerializer;
    fn into_serializer(self) -> Self::S {
        StringSerializer::new(self.to_string())
    }

    fn size(&self) -> Option<usize> {
        Some(crate::serde::escaped_string_size(&self.to_string()) + 2)
    }
}

impl IntoSerializer for std::path::PathBuf {
    type S = StringSerializer;
    fn into_serializer(self) -> Self::S {
        StringSerializer::new(self.to_string_lossy().into_owned())
    }

    fn size(&self) -> Option<usize> {
        let s = self.to_string_lossy();
        Some(crate::serde::escaped_string_size(&s) + 2)
    }
}

impl IntoSerializer for std::time::Duration {
    type S = U64Serializer;
    fn into_serializer(self) -> Self::S {
        U64Serializer::new(self.as_secs())
    }

    fn size(&self) -> Option<usize> {
        Some(self.as_secs().to_string().len())
    }
}

impl IntoSerializer for std::time::SystemTime {
    type S = StringSerializer;
    fn into_serializer(self) -> Self::S {
        let dur = self
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        StringSerializer::new(dur.as_secs_f64().to_string())
    }

    fn size(&self) -> Option<usize> {
        let dur = self
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        Some(crate::serde::escaped_string_size(&dur.as_secs_f64().to_string()) + 2)
    }
}

pub struct OptionSerializer<T: IntoSerializer> {
    inner: Option<T::S>,
    emitted: bool,
}

impl<T: IntoSerializer> OptionSerializer<T> {
    pub fn new(value: Option<T>) -> Self {
        Self {
            inner: value.map(IntoSerializer::into_serializer),
            emitted: false,
        }
    }
}

impl<T: IntoSerializer + Unpin> Serializer for OptionSerializer<T>
where
    T::S: Unpin,
{
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        if self.emitted {
            return Poll::Ready(None);
        }
        match self.inner.as_mut() {
            Some(serializer) => match serializer.poll(cx) {
                Poll::Ready(Some(Ok(bytes))) => Poll::Ready(Some(Ok(bytes))),
                Poll::Ready(Some(Err(e))) => {
                    self.emitted = true;
                    Poll::Ready(Some(Err(e)))
                }
                Poll::Ready(None) => {
                    self.emitted = true;
                    Poll::Ready(None)
                }
                Poll::Pending => Poll::Pending,
            },
            None => {
                self.emitted = true;
                Poll::Ready(Some(Ok("null".into())))
            }
        }
    }
}

impl<T: IntoSerializer + Unpin> Unpin for OptionSerializer<T> where T::S: Unpin {}

impl<T: IntoSerializer + Unpin> IntoSerializer for Option<T>
where
    T::S: Unpin,
{
    type S = OptionSerializer<T>;

    fn into_serializer(self) -> Self::S {
        OptionSerializer::new(self)
    }

    fn size(&self) -> Option<usize> {
        match self {
            Some(value) => value.size(),
            None => Some(4),
        }
    }
}

pub struct BoxSerializer<T: IntoSerializer> {
    inner: T::S,
}

impl<T: IntoSerializer> BoxSerializer<T> {
    pub fn new(value: Box<T>) -> Self {
        Self {
            inner: (*value).into_serializer(),
        }
    }
}

impl<T: IntoSerializer + Unpin> Serializer for BoxSerializer<T>
where
    T::S: Unpin,
{
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        self.inner.poll(cx)
    }
}

impl<T: IntoSerializer + Unpin> Unpin for BoxSerializer<T> where T::S: Unpin {}

impl<T: IntoSerializer + Unpin> IntoSerializer for Box<T>
where
    T::S: Unpin,
{
    type S = BoxSerializer<T>;

    fn into_serializer(self) -> Self::S {
        BoxSerializer::new(self)
    }

    fn size(&self) -> Option<usize> {
        (**self).size()
    }
}

impl IntoSerializer for std::net::Ipv4Addr {
    type S = StringSerializer;
    fn into_serializer(self) -> Self::S {
        StringSerializer::new(self.to_string())
    }

    fn size(&self) -> Option<usize> {
        Some(crate::serde::escaped_string_size(&self.to_string()) + 2)
    }
}

impl IntoSerializer for std::net::Ipv6Addr {
    type S = StringSerializer;
    fn into_serializer(self) -> Self::S {
        StringSerializer::new(self.to_string())
    }

    fn size(&self) -> Option<usize> {
        Some(crate::serde::escaped_string_size(&self.to_string()) + 2)
    }
}

enum StringState {
    Start,
    Streaming { chunk_start: usize },
    Closing,
    Done,
}

/// Serializer for strings with automatic chunking.
///
/// For strings ≤ 128KB ([`CHUNK_SIZE`](crate::CHUNK_SIZE)), emits the entire
/// quoted string as a single chunk.
///
/// For strings > 128KB, emits the string in multiple chunks:
/// 1. `"` + first 128KB of escaped string
/// 2. subsequent 128KB chunks
/// 3. closing `"` (optional, may be combined with last data chunk)
///
/// # Large String Example
///
/// ```
/// use stream_json::serializers::StringSerializer;
///
/// let large = "x".repeat(200_000); // 200KB
/// let ser = StringSerializer::new(large);
/// // First chunk: "\"xxx...xxx" (128KB + opening quote)
/// // Second chunk: "xxx...xxx\"" (72KB + closing quote)
/// ```
pub struct StringSerializer {
    data: String,
    state: StringState,
}

impl StringSerializer {
    /// Creates a new string serializer.
    pub fn new(data: String) -> Self {
        Self {
            data,
            state: StringState::Start,
        }
    }
}

impl Serializer for StringSerializer {
    fn poll(&mut self, _cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        match &mut self.state {
            StringState::Start => {
                let escaped = escape_string(&self.data);
                if escaped.len() <= CHUNK_SIZE {
                    self.state = StringState::Done;
                    return Poll::Ready(Some(Ok(format!("\"{}\"", escaped).into())));
                }
                self.data = escaped;
                let end = std::cmp::min(CHUNK_SIZE, self.data.len());
                let chunk = self.data[..end].to_string();
                self.state = StringState::Streaming { chunk_start: end };
                return Poll::Ready(Some(Ok(format!("\"{}", chunk).into())));
            }
            StringState::Streaming { chunk_start } => {
                if *chunk_start >= self.data.len() {
                    self.state = StringState::Closing;
                    return Poll::Ready(Some(Ok("\"".into())));
                }
                let end = std::cmp::min(*chunk_start + CHUNK_SIZE, self.data.len());
                let chunk = self.data[*chunk_start..end].to_string();
                *chunk_start = end;
                if *chunk_start >= self.data.len() {
                    self.state = StringState::Done;
                    return Poll::Ready(Some(Ok(format!("{}\"", chunk).into())));
                }
                return Poll::Ready(Some(Ok(chunk.into())));
            }
            StringState::Closing => {
                self.state = StringState::Done;
                Poll::Ready(Some(Ok("\"".into())))
            }
            StringState::Done => Poll::Ready(None),
        }
    }
}

impl Unpin for StringSerializer {}

impl IntoSerializer for String {
    type S = StringSerializer;
    fn into_serializer(self) -> Self::S {
        StringSerializer::new(self)
    }

    fn size(&self) -> Option<usize> {
        Some(crate::serde::escaped_string_size(self) + 2)
    }
}

impl IntoSerializer for &str {
    type S = StringSerializer;
    fn into_serializer(self) -> Self::S {
        StringSerializer::new(self.to_string())
    }

    fn size(&self) -> Option<usize> {
        Some(crate::serde::escaped_string_size(self) + 2)
    }
}

enum VecState {
    Start,
    EmitOpenBracket,
    Serializing { idx: usize },
    EmitComma { next_idx: usize },
    EmitCloseBracket,
    End,
}

/// Serializer for `Vec<T>` that emits a JSON array.
///
/// Each element is serialized via its own serializer obtained by calling
/// `into_serializer()` on each element. Commas are automatically inserted
/// between elements.
///
/// # Example
///
/// ```
/// use stream_json::serializers::VecSerializer;
///
/// let ser = VecSerializer::new(vec![1, 2, 3]);
/// // Output chunks: "[" → "1" → "," → "2" → "," → "3" → "]"
/// ```
pub struct VecSerializer<T: IntoSerializer> {
    serializers: Vec<T::S>,
    state: VecState,
}

impl<T: IntoSerializer> VecSerializer<T> {
    /// Creates a new vec serializer from a vector of items.
    ///
    /// Each item is immediately converted to its serializer form.
    pub fn new(items: Vec<T>) -> Self {
        let serializers: Vec<T::S> = items.into_iter().map(|i| i.into_serializer()).collect();
        Self {
            serializers,
            state: VecState::Start,
        }
    }
}

impl<T: IntoSerializer + Unpin> Serializer for VecSerializer<T> {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        loop {
            match &mut self.state {
                VecState::Start => {
                    if self.serializers.is_empty() {
                        self.state = VecState::End;
                        return Poll::Ready(Some(Ok("[]".into())));
                    }
                    self.state = VecState::EmitOpenBracket;
                }
                VecState::EmitOpenBracket => {
                    self.state = VecState::Serializing { idx: 0 };
                    return Poll::Ready(Some(Ok("[".into())));
                }
                VecState::Serializing { idx } => {
                    let serializer = &mut self.serializers[*idx];
                    match serializer.poll(cx) {
                        Poll::Ready(Some(result)) => return Poll::Ready(Some(result)),
                        Poll::Ready(None) => {
                            let next_idx = *idx + 1;
                            if next_idx >= self.serializers.len() {
                                self.state = VecState::EmitCloseBracket;
                            } else {
                                self.state = VecState::EmitComma { next_idx };
                            }
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                VecState::EmitComma { next_idx } => {
                    self.state = VecState::Serializing { idx: *next_idx };
                    return Poll::Ready(Some(Ok(",".into())));
                }
                VecState::EmitCloseBracket => {
                    self.state = VecState::End;
                    return Poll::Ready(Some(Ok("]".into())));
                }
                VecState::End => return Poll::Ready(None),
            }
        }
    }
}

impl<T: IntoSerializer + Unpin> Unpin for VecSerializer<T> {}

impl<T: IntoSerializer + Unpin> IntoSerializer for Vec<T>
where
    T::S: Unpin,
{
    type S = VecSerializer<T>;
    fn into_serializer(self) -> Self::S {
        VecSerializer::new(self)
    }

    fn size(&self) -> Option<usize> {
        let mut total = 2;
        for (idx, item) in self.iter().enumerate() {
            total += item.size()?;
            if idx + 1 < self.len() {
                total += 1;
            }
        }
        Some(total)
    }
}

enum DynArrayState {
    Start,
    EmitValue,
    EmitComma,
    Close,
    End,
}

pub struct DynArraySerializer {
    items: Vec<Box<dyn Serializer + Unpin>>,
    idx: usize,
    state: DynArrayState,
}

impl DynArraySerializer {
    pub fn new(items: Vec<Box<dyn Serializer + Unpin>>) -> Self {
        Self {
            items,
            idx: 0,
            state: DynArrayState::Start,
        }
    }
}

impl Serializer for DynArraySerializer {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        loop {
            match self.state {
                DynArrayState::Start => {
                    if self.items.is_empty() {
                        self.state = DynArrayState::End;
                        return Poll::Ready(Some(Ok("[]".into())));
                    }
                    self.state = DynArrayState::EmitValue;
                    return Poll::Ready(Some(Ok("[".into())));
                }
                DynArrayState::EmitValue => {
                    let serializer = &mut self.items[self.idx];
                    match serializer.poll(cx) {
                        Poll::Ready(Some(result)) => return Poll::Ready(Some(result)),
                        Poll::Ready(None) => {
                            self.idx += 1;
                            if self.idx >= self.items.len() {
                                self.state = DynArrayState::Close;
                            } else {
                                self.state = DynArrayState::EmitComma;
                            }
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                DynArrayState::EmitComma => {
                    self.state = DynArrayState::EmitValue;
                    return Poll::Ready(Some(Ok(",".into())));
                }
                DynArrayState::Close => {
                    self.state = DynArrayState::End;
                    return Poll::Ready(Some(Ok("]".into())));
                }
                DynArrayState::End => return Poll::Ready(None),
            }
        }
    }
}

impl Unpin for DynArraySerializer {}

enum DynObjectState {
    Start,
    EmitKey,
    EmitValue,
    EmitComma,
    Close,
    End,
}

pub struct DynObjectSerializer {
    fields: Vec<(Bytes, Box<dyn Serializer + Unpin>)>,
    idx: usize,
    state: DynObjectState,
}

impl DynObjectSerializer {
    pub fn new(fields: Vec<(Bytes, Box<dyn Serializer + Unpin>)>) -> Self {
        Self {
            fields,
            idx: 0,
            state: DynObjectState::Start,
        }
    }
}

impl Serializer for DynObjectSerializer {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        loop {
            match self.state {
                DynObjectState::Start => {
                    if self.fields.is_empty() {
                        self.state = DynObjectState::End;
                        return Poll::Ready(Some(Ok("{}".into())));
                    }
                    self.state = DynObjectState::EmitKey;
                    return Poll::Ready(Some(Ok("{".into())));
                }
                DynObjectState::EmitKey => {
                    let (key, _) = &self.fields[self.idx];
                    self.state = DynObjectState::EmitValue;
                    return Poll::Ready(Some(Ok(key.clone())));
                }
                DynObjectState::EmitValue => {
                    let (_, serializer) = &mut self.fields[self.idx];
                    match serializer.poll(cx) {
                        Poll::Ready(Some(result)) => return Poll::Ready(Some(result)),
                        Poll::Ready(None) => {
                            self.idx += 1;
                            if self.idx >= self.fields.len() {
                                self.state = DynObjectState::Close;
                            } else {
                                self.state = DynObjectState::EmitComma;
                            }
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                DynObjectState::EmitComma => {
                    self.state = DynObjectState::EmitKey;
                    return Poll::Ready(Some(Ok(",".into())));
                }
                DynObjectState::Close => {
                    self.state = DynObjectState::End;
                    return Poll::Ready(Some(Ok("}".into())));
                }
                DynObjectState::End => return Poll::Ready(None),
            }
        }
    }
}

impl Unpin for DynObjectSerializer {}

impl Serializer for Box<dyn Serializer + Unpin> {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        (**self).poll(cx)
    }
}

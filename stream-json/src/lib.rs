//! # stream-json
//!
//! A **streaming, async-only** JSON serialization framework for Rust. Designed to
//! handle strings up to 1TB without loading into memory.
//!
//! ## Core Features
//!
//! - **Async-first**: No sync serialization. Uses `poll` interface for integration
//!   with `futures`.
//! - **Streaming**: Serializes data in chunks (128KB default) to avoid memory
//!   exhaustion.
//! - **Token-based**: [`Token`] enum for structured serialization to JSON tokens.
//! - **Derive macros**: `#[derive(Serialize)]` for structs and enums via
//!   `stream-json-macros`.
//!
//! ## Quick Start
//!
//! ```
//! use stream_json::{Serializer, IntoSerializer};
//!
//! // Simple value
//! let serializer = 42i64.into_serializer();
//! // serializer is I64Serializer
//!
//! // String
//! let serializer = "hello".to_string().into_serializer();
//! // serializer is StringSerializer
//!
//! // Vec
//! let serializer = vec![1, 2, 3].into_serializer();
//! // serializer is VecSerializer<i32>
//! ```
//!
//! ## Architecture
//!
//! ```
//! // stream-json
//! //
//! // serde.rs
//! // ├── Token (enum)           JSON tokens
//! // ├── Serializer (trait)    poll-based interface
//! // ├── IntoSerializer        conversion trait
//! // ├── IntoStreamSerializer  stream wrapper
//! // └── TokenSerializer       tokens to bytes
//! //
//! // std_impl.rs
//! // ├── UnitSerializer         ()
//! // ├── BoolSerializerState    bool
//! // ├── I64Serializer          integer types
//! // ├── U64Serializer          unsigned types
//! // ├── F64Serializer          float types
//! // ├── StringSerializer       strings (chunked)
//! // └── VecSerializer          Vec<T>
//! //
//! // stream-json-macros
//! // └── #[derive(Serialize)]  struct/enum derive
//! ```
//!
//! ## Serializers Module
//!
//! This module is the public API for [`std_impl`]. All serializers can be created
//! directly or via the [`IntoSerializer`] trait.
//!
//! ### Value Serializers
//!
//! | Type | Serializer | Output |
//! |------|-------------|--------|
//! | `()` | [`UnitSerializer`] | `null` |
//! | `bool` | [`BoolSerializerState`] | `true`/`false` |
//! | `i64`, `i8`-`i32` | [`I64Serializer`] | JSON number |
//! | `u64`, `u8`-`u32` | [`U64Serializer`] | JSON number |
//! | `f64`, `f32` | [`F64Serializer`] | JSON number (NaN/Inf → `null`) |
//!
//! ### String Serializer
//!
//! [`StringSerializer`] handles strings with chunking for large content:
//!
//! - Strings ≤ 128KB: emitted as single chunk `"..."`
//! - Strings > 128KB: emitted in 128KB chunks with quote prefix/suffix
//!
//! ### Collection Serializer
//!
//! [`VecSerializer<T>`] serializes `Vec<T>` as JSON array. Each element is
//! serialized via its own serializer, with proper comma handling.
//!
//! ## Token Module
//!
//! The [`Token`] enum represents JSON tokens for token-based serialization:
//!
//! - Structure: [`Token::StartArray`], [`Token::EndArray`],
//!   [`Token::StartObject`], [`Token::EndObject`]
//! - Values: [`Token::Null`], [`Token::Bool`], [`Token::I64`], [`Token::U64`],
//!   [`Token::F64`], [`Token::String`]
//! - Object members: [`Token::Key`], [`Token::Comma`], [`Token::Colon`]
//!
//! Use [`TokenSerializer`] to convert a slice of tokens into bytes.

pub const CHUNK_SIZE: usize = 128 * 1024;

pub mod error;
pub mod serde;
pub mod std_impl;

#[cfg(feature = "base64")]
pub mod base64_embed;

#[cfg(test)]
pub mod tests;

pub use error::Error;
pub use serde::{
    IntoSerializer, IntoStreamSerializer, IntoStreamSerializer as StreamSerializer, Serializer,
    Token, TokenSerializer,
};

pub use stream_json_macros::Serialize;

#[cfg(feature = "base64")]
pub use base64_embed::Base64EmbedFile;

pub mod serializers {
    //! Built-in serializers for primitive and standard library types.
    //!
    //! All serializers implement [`Serializer`](super::Serializer) and can be
    //! created directly or via the [`IntoSerializer`](super::IntoSerializer) trait.
    //!
    //! ## Example
    //!
    //! ```
    //! use stream_json::serializers::{StringSerializer, VecSerializer};
    //!
    //! // Direct construction
    //! let ser = StringSerializer::new("hello".to_string());
    //! ```

    pub use super::std_impl::*;

    #[cfg(feature = "base64")]
    pub use super::base64_embed::Base64EmbedFile;
}

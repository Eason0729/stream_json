//! # stream-json-macros
//!
//! Procedural macros for [`stream-json`](https://docs.rs/stream-json), specifically the
//! `#[derive(IntoSerializer)]` macro.
//!
//! See the [`Serialize`] derive macro for usage details.

mod attributes;
mod derive;
mod ident;

#[proc_macro_derive(IntoSerializer, attributes(stream))]
pub fn derive_into_serializer(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive::derive_into_serializer(item)
}

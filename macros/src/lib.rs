//! # stream-json-macros
//!
//! Procedural macros for [`stream-json`](https://docs.rs/stream-json), specifically the
//! `#[derive(Serialize)]` macro.
//!
//! See the [`Serialize`] derive macro for usage details.

mod attributes;
mod derive;
mod ident;

#[proc_macro_derive(Serialize, attributes(stream))]
pub fn derive_serialize(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive::derive_serialize(item)
}

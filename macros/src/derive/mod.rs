use proc_macro::TokenStream;
use syn::{Data, DeriveInput, Fields};

mod shared;
mod state_machine;

pub fn derive_struct(name: &syn::Ident, fields: &Fields, vis: &syn::Visibility) -> TokenStream {
    state_machine::build_struct(name, fields, vis).into()
}

pub fn derive_enum(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>,
    vis: &syn::Visibility,
) -> TokenStream {
    state_machine::build_enum(name, variants, vis).into()
}

pub fn derive_into_serializer(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as DeriveInput);
    let name = &input.ident;

    match &input.data {
        Data::Struct(data) => derive_struct(name, &data.fields, &input.vis),
        Data::Enum(data) => derive_enum(name, &data.variants, &input.vis),
        Data::Union(_) => TokenStream::from(
            syn::Error::new_spanned(input, "Union types are not supported").into_compile_error(),
        ),
    }
}

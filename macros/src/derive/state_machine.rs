use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident, Visibility};

use super::super::attributes::get_variant_rename;
use super::super::ident::{serializer_name, state_name};
use super::shared::{
    escape_json_string, escaped_string_size, generate_emit_key_arms,
    generate_emit_key_or_comma_arms, generate_emit_value_arms, generate_field_keys,
    generate_into_serializer_arm, generate_serializer_struct,
};

fn to_snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (idx, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if idx > 0 {
                out.push('_');
            }
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn build_struct(name: &Ident, fields: &Fields, vis: &Visibility) -> TokenStream {
    let serializer_name = serializer_name(name);
    let field_count = match fields {
        Fields::Named(f) => f.named.len(),
        Fields::Unnamed(f) => f.unnamed.len(),
        Fields::Unit => 0,
    };

    if field_count == 0 {
        return quote! {
            impl stream_json::serde::IntoSerializer for #name {
                type S = #serializer_name;

                fn into_serializer(self) -> Self::S {
                    #serializer_name { emitted: false }
                }

                fn size(&self) -> Option<usize> {
                    Some(2)
                }
            }

            struct #serializer_name {
                emitted: bool,
            }

            impl stream_json::serde::Serializer for #serializer_name {
                fn poll(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Result<bytes::Bytes, stream_json::error::Error>>> {
                    if self.emitted {
                        std::task::Poll::Ready(None)
                    } else {
                        self.emitted = true;
                        std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"{}"))))
                    }
                }
            }

            impl std::marker::Unpin for #serializer_name {}
        };
    }

    match fields {
        Fields::Unnamed(_) if field_count == 1 => {
            build_tuple_struct_single_field(name, fields, vis)
        }
        Fields::Unnamed(_) => build_tuple_struct_multiple_fields(name, fields, vis),
        Fields::Named(_) => build_named_struct(name, fields, vis),
        Fields::Unit => {
            quote! {
                impl stream_json::serde::IntoSerializer for #name {
                    type S = #serializer_name;
                    fn into_serializer(self) -> Self::S {
                        #serializer_name { emitted: false }
                    }
                    fn size(&self) -> Option<usize> {
                        Some(2)
                    }
                }
                struct #serializer_name {
                    emitted: bool,
                }
                impl stream_json::serde::Serializer for #serializer_name {
                    fn poll(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Result<bytes::Bytes, stream_json::error::Error>>> {
                        if self.emitted {
                            std::task::Poll::Ready(None)
                        } else {
                            self.emitted = true;
                            std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"{}"))))
                        }
                    }
                }
                impl std::marker::Unpin for #serializer_name {}
            }
        }
    }
}

fn build_tuple_struct_single_field(
    name: &Ident,
    _fields: &Fields,
    _vis: &Visibility,
) -> TokenStream {
    let serializer_name = serializer_name(name);
    let idx = syn::Index::from(0);
    quote! {
        impl stream_json::serde::IntoSerializer for #name {
            type S = #serializer_name;
            fn into_serializer(self) -> Self::S {
                #serializer_name {
                    inner: ::std::boxed::Box::new(self.#idx.into_serializer()) as ::std::boxed::Box<dyn stream_json::serde::Serializer + Unpin>,
                }
            }
            fn size(&self) -> Option<usize> {
                self.0.size()
            }
        }
        struct #serializer_name {
            inner: ::std::boxed::Box<dyn stream_json::serde::Serializer + Unpin>,
        }
        impl stream_json::serde::Serializer for #serializer_name {
            fn poll(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Result<bytes::Bytes, stream_json::error::Error>>> {
                self.inner.poll(cx)
            }
        }
        impl std::marker::Unpin for #serializer_name {}
    }
}

fn build_tuple_struct_multiple_fields(
    name: &Ident,
    fields: &Fields,
    _vis: &Visibility,
) -> TokenStream {
    let serializer_name = serializer_name(name);
    let count = match fields {
        Fields::Unnamed(f) => f.unnamed.len(),
        _ => 0,
    };
    let indices: Vec<_> = (0..count).map(syn::Index::from).collect();
    let values: Vec<_> = indices
        .iter()
        .map(|idx| {
            quote! {
                ::std::boxed::Box::new(self.#idx.into_serializer()) as ::std::boxed::Box<dyn stream_json::serde::Serializer + Unpin>
            }
        })
        .collect();
    let size_parts: Vec<_> = indices
        .iter()
        .map(|idx| {
            quote! {
                {
                    let size = self.#idx.size()?;
                    if !first { total += 1; }
                    total += size;
                    first = false;
                }
            }
        })
        .collect();
    quote! {
        impl stream_json::serde::IntoSerializer for #name {
            type S = #serializer_name;
            fn into_serializer(self) -> Self::S {
                #serializer_name {
                    inner: stream_json::std_impl::DynArraySerializer::new(vec![
                        #(#values),*
                    ]),
                }
            }
            fn size(&self) -> Option<usize> {
                let mut total = 2usize;
                let mut first = true;
                #(#size_parts)*
                Some(total)
            }
        }
        struct #serializer_name {
            inner: stream_json::std_impl::DynArraySerializer,
        }
        impl stream_json::serde::Serializer for #serializer_name {
            fn poll(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Result<bytes::Bytes, stream_json::error::Error>>> {
                self.inner.poll(cx)
            }
        }
        impl std::marker::Unpin for #serializer_name {}
    }
}

fn build_named_struct(name: &Ident, fields: &Fields, vis: &Visibility) -> TokenStream {
    let (field_infos, keys_array) = generate_field_keys(fields, name.clone());
    let field_count = field_infos.len();
    let state_name = state_name(name);
    let serializer_name = serializer_name(name);

    let size_body = {
        let mut parts = Vec::new();
        for fi in &field_infos {
            let field_value = {
                let ident = fi.ident.as_ref().expect("named field");
                quote! { &self.#ident }
            };
            let include = if let Some(skip_expr) = &fi.skip_serialize_if {
                quote! { !(#skip_expr)(#field_value) }
            } else {
                quote! { true }
            };
            let key_size = fi.key_size;
            parts.push(quote! {
                if #include {
                    let field_size = match stream_json::serde::IntoSerializer::size(#field_value) {
                        Some(size) => size,
                        None => return None,
                    };
                    if !first {
                        total += 1;
                    }
                    total += #key_size + field_size;
                    first = false;
                }
            });
        }
        quote! {
            let mut total = 2usize;
            let mut first = true;
            #(#parts)*
            Some(total)
        }
    };

    let into_serializer_arm =
        generate_into_serializer_arm(name, fields, &field_infos, &keys_array, Some(&size_body));
    let (serializer_struct, unpin_impl) =
        generate_serializer_struct(name, vis, fields, &field_infos);
    let emit_key_arms = generate_emit_key_arms(name, &field_infos);
    let emit_value_arms = generate_emit_value_arms(name, &field_infos, field_count);
    let emit_key_or_comma_arms = generate_emit_key_or_comma_arms(name, &field_infos);

    quote! {
        #into_serializer_arm

        #serializer_struct

        enum #state_name {
            Start,
            EmitKey,
            EmitValue,
            EmitKeyOrComma,
            EmitComma,
            ClosingBrace,
            Done,
        }

        impl stream_json::serde::Serializer for #serializer_name {
            fn poll(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Result<bytes::Bytes, stream_json::error::Error>>> {
                loop {
                    match &mut self.state {
                        #state_name::Start => {
                            self.state = #state_name::EmitKey;
                            return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"{"))));
                        }
                        #state_name::EmitKey => {
                            match self.field_idx {
                                #(#emit_key_arms)*
                                _ => {
                                    self.state = #state_name::ClosingBrace;
                                    continue;
                                }
                            }
                        }
                        #state_name::EmitValue => {
                            match self.field_idx {
                                #(#emit_value_arms)*
                                _ => {
                                    self.state = #state_name::ClosingBrace;
                                    continue;
                                }
                            }
                        }
                        #state_name::EmitKeyOrComma => {
                            match self.field_idx {
                                #(#emit_key_or_comma_arms)*
                                _ => {
                                    self.state = #state_name::ClosingBrace;
                                    continue;
                                }
                            }
                        }
                        #state_name::EmitComma => {
                            self.state = #state_name::EmitKey;
                            return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b","))));
                        }
                        #state_name::ClosingBrace => {
                            self.state = #state_name::Done;
                            return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"}"))));
                        }
                        #state_name::Done => return std::task::Poll::Ready(None),
                    }
                }
            }
        }

        #unpin_impl
    }
}

pub fn build_enum(
    name: &Ident,
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>,
    vis: &Visibility,
) -> TokenStream {
    let serializer_name = serializer_name(name);
    let state_name = state_name(name);

    let mut variant_serializers = Vec::new();
    let mut variant_sizes = Vec::new();
    for variant in variants {
        let ident = &variant.ident;
        let variant_name =
            get_variant_rename(variant).unwrap_or_else(|| to_snake_case(&ident.to_string()));
        let escaped_variant_name = escape_json_string(&variant_name);
        let variant_key = format!("\"{}\"", escaped_variant_name);
        let variant_key_len = escaped_string_size(&variant_name) + 2;
        match &variant.fields {
            Fields::Unit => {
                variant_serializers.push(quote! {
                    #name::#ident => {
                        return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(#variant_key.as_bytes()))));
                    }
                });
                variant_sizes.push(quote! { #name::#ident => Some(#variant_key_len), });
            }
            Fields::Unnamed(_) => {
                variant_serializers.push(quote! {
                    #name::#ident(..) => {
                        return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(#variant_key.as_bytes()))));
                    }
                });
                variant_sizes.push(quote! { #name::#ident(..) => Some(#variant_key_len), });
            }
            Fields::Named(_) => {
                variant_serializers.push(quote! {
                    #name::#ident { .. } => {
                        return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(#variant_key.as_bytes()))));
                    }
                });
                variant_sizes.push(quote! { #name::#ident { .. } => Some(#variant_key_len), });
            }
        }
    }

    quote! {
        impl stream_json::serde::IntoSerializer for #name {
            type S = #serializer_name;
            fn into_serializer(self) -> Self::S {
                #serializer_name {
                    inner: self,
                    state: #state_name::Start,
                }
            }

            fn size(&self) -> Option<usize> {
                match self {
                    #(#variant_sizes)*
                }
            }
        }

        #vis struct #serializer_name {
            inner: #name,
            state: #state_name,
        }

        enum #state_name {
            Start,
            Done,
        }

        impl stream_json::serde::Serializer for #serializer_name {
            fn poll(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Result<bytes::Bytes, stream_json::error::Error>>> {
                loop {
                    match &mut self.state {
                        #state_name::Start => {
                            self.state = #state_name::Done;
                            match &mut self.inner {
                                #(#variant_serializers)*
                            }
                        }
                        #state_name::Done => {
                            return std::task::Poll::Ready(None);
                        }
                    }
                }
            }
        }

        impl std::marker::Unpin for #serializer_name {}
    }
}

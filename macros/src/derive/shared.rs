use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident, Index, Type, Visibility};

use super::super::attributes::get_field_rename;
use super::super::ident::{field_name, serializer_name, state_name};

pub struct FieldInfo {
    pub index: usize,
    pub ident: Option<Ident>,
    pub ty: Type,
    pub skip_serialize_if: Option<TokenStream>,
    pub key_bytes: TokenStream,
    pub key_size: usize,
}

fn escaped_string_size(s: &str) -> usize {
    s.chars()
        .map(|c| match c {
            '"' | '\\' | '\n' | '\r' | '\t' => 2,
            c if c.is_control() => 6,
            c => c.len_utf8(),
        })
        .sum()
}

fn escape_json_string(s: &str) -> String {
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

fn key_size(key: &str) -> usize {
    escaped_string_size(key) + 3
}

pub fn generate_field_keys(fields: &Fields, _span: Ident) -> (Vec<FieldInfo>, TokenStream) {
    match fields {
        Fields::Named(fields) => {
            let field_infos: Vec<FieldInfo> = fields
                .named
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let ident = f.ident.as_ref().cloned().expect("named field has ident");
                    let key = get_field_rename(f).unwrap_or_else(|| ident.to_string());
                    let escaped_key = escape_json_string(&key);
                    let key_str = format!("\"{}\":", escaped_key);
                    FieldInfo {
                        index: i,
                        ident: Some(ident),
                        ty: f.ty.clone(),
                        skip_serialize_if: super::super::attributes::get_skip_serialize_if(f),
                        key_bytes: quote! { bytes::Bytes::from(#key_str) },
                        key_size: key_size(&key),
                    }
                })
                .collect();
            let keys: Vec<_> = field_infos.iter().map(|fi| fi.key_bytes.clone()).collect();
            (field_infos, quote! { [#(#keys,)*] })
        }
        Fields::Unnamed(fields) => {
            let field_infos: Vec<FieldInfo> = fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let key_str = format!("\"{}\":", i);
                    FieldInfo {
                        index: i,
                        ident: None,
                        ty: fields.unnamed[i].ty.clone(),
                        skip_serialize_if: super::super::attributes::get_skip_serialize_if(
                            &fields.unnamed[i],
                        ),
                        key_bytes: quote! { bytes::Bytes::from(#key_str) },
                        key_size: key_size(&i.to_string()),
                    }
                })
                .collect();
            let keys: Vec<_> = field_infos.iter().map(|fi| fi.key_bytes.clone()).collect();
            (field_infos, quote! { [#(#keys,)*] })
        }
        Fields::Unit => (vec![], quote! { [] }),
    }
}

pub fn generate_field_defs_and_inits(
    name: &Ident,
    fields: &Fields,
    field_infos: &[FieldInfo],
) -> (TokenStream, TokenStream) {
    let field_count = field_infos.len();
    if field_count == 0 {
        return (quote! {}, quote! {});
    }

    let field_defs: Vec<_> = field_infos
        .iter()
        .map(|fi| {
            let fname = field_name(fi.index, name.span());
            let ty = &fi.ty;
            quote! { #fname: stream_json::serde::FieldState<#ty> }
        })
        .collect();

    let field_inits: Vec<TokenStream> = field_infos
        .iter()
        .map(|fi| {
            let fname = field_name(fi.index, name.span());
            match fields {
                Fields::Named(_) => {
                    let ident = fi.ident.as_ref().expect("named field");
                    let skip_if = fi
                        .skip_serialize_if
                        .as_ref()
                        .map(|skip_expr| quote! { Some(Box::new(#skip_expr)) })
                        .unwrap_or_else(|| quote! { None });
                    quote! {
                        #fname: stream_json::serde::FieldState::Waiting {
                            value: Some(self.#ident),
                            skip_if: #skip_if,
                        }
                    }
                }
                Fields::Unnamed(_) => {
                    let idx = Index::from(fi.index);
                    let skip_if = fi
                        .skip_serialize_if
                        .as_ref()
                        .map(|skip_expr| quote! { Some(Box::new(#skip_expr)) })
                        .unwrap_or_else(|| quote! { None });
                    quote! {
                        #fname: stream_json::serde::FieldState::Waiting {
                            value: Some(self.#idx),
                            skip_if: #skip_if,
                        }
                    }
                }
                Fields::Unit => quote! {},
            }
        })
        .collect();

    (quote! { #(#field_defs,)* }, quote! { #(#field_inits,)* })
}

pub fn generate_emit_key_arms(name: &Ident, field_infos: &[FieldInfo]) -> Vec<TokenStream> {
    let state_name = state_name(name);
    field_infos
        .iter()
        .map(|fi| {
            let i = fi.index;
            let fname = field_name(fi.index, name.span());
            quote! {
                #i => {
                    let already_active = matches!(&self.#fname, stream_json::serde::FieldState::Active(_));
                    if !already_active && !self.#fname.prepare() {
                        if self.field_idx + 1 < self.keys.len() {
                            self.field_idx += 1;
                            self.state = #state_name::EmitKey;
                        } else {
                            self.state = #state_name::ClosingBrace;
                        }
                        continue;
                    }
                    let key = self.keys[#i].clone();
                    self.state = #state_name::EmitValue;
                    return std::task::Poll::Ready(Some(Ok(key)));
                }
            }
        })
        .collect()
}

pub fn generate_emit_value_arms(
    name: &Ident,
    field_infos: &[FieldInfo],
    field_count: usize,
) -> Vec<TokenStream> {
    let state_name = state_name(name);
    field_infos
        .iter()
        .map(|fi| {
            let i = fi.index;
            let fname = field_name(fi.index, name.span());
            quote! {
                #i => {
                    let serializer = &mut self.#fname;
                    match serializer.poll(cx) {
                        std::task::Poll::Ready(Some(result)) => {
                            return std::task::Poll::Ready(Some(result));
                        }
                        std::task::Poll::Ready(None) => {
                            self.#fname = stream_json::serde::FieldState::Dropped;
                            if self.field_idx + 1 < #field_count {
                                self.field_idx += 1;
                                self.state = #state_name::EmitKeyOrComma;
                                continue;
                            } else {
                                self.state = #state_name::ClosingBrace;
                                continue;
                            }
                        }
                        std::task::Poll::Pending => {
                            return std::task::Poll::Pending;
                        }
                    }
                }
            }
        })
        .collect()
}

pub fn generate_emit_key_or_comma_arms(
    name: &Ident,
    field_infos: &[FieldInfo],
) -> Vec<TokenStream> {
    let state_name = state_name(name);
    field_infos
        .iter()
        .map(|fi| {
            let i = fi.index;
            let fname = field_name(fi.index, name.span());
            quote! {
                #i => {
                    if self.field_idx >= self.keys.len() {
                        self.state = #state_name::ClosingBrace;
                        return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b""))));
                    }
                    if !self.#fname.prepare() {
                        if self.field_idx + 1 < self.keys.len() {
                            self.field_idx += 1;
                            self.state = #state_name::EmitKeyOrComma;
                            continue;
                        } else {
                            self.state = #state_name::ClosingBrace;
                            return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b""))));
                        }
                    }
                    if self.field_idx > 0 {
                        self.state = #state_name::EmitKey;
                        return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b","))));
                    } else {
                        self.state = #state_name::EmitKey;
                    }
                }
            }
        })
        .collect()
}

pub fn generate_into_serializer_arm(
    name: &Ident,
    fields: &Fields,
    field_infos: &[FieldInfo],
    keys_array: &TokenStream,
    size_body: Option<&TokenStream>,
) -> TokenStream {
    let serializer_name = serializer_name(name);
    let state_name = state_name(name);
    let (_, field_inits) = generate_field_defs_and_inits(name, fields, field_infos);
    let size_impl = size_body
        .map(|body| quote! { fn size(&self) -> Option<usize> { #body } })
        .unwrap_or_else(|| quote! {});

    quote! {
        impl stream_json::serde::IntoSerializer for #name {
            type S = #serializer_name;
            fn into_serializer(self) -> Self::S {
                #serializer_name {
                    #field_inits
                    keys: #keys_array,
                    field_idx: 0,
                    state: #state_name::Start,
                }
            }

            #size_impl
        }
    }
}

pub fn generate_serializer_struct(
    name: &Ident,
    vis: &Visibility,
    fields: &Fields,
    field_infos: &[FieldInfo],
) -> (TokenStream, TokenStream) {
    let serializer_name = serializer_name(name);
    let state_name = state_name(name);
    let field_count = field_infos.len();

    let (field_defs, _) = generate_field_defs_and_inits(name, fields, field_infos);

    let struct_fields = if field_count == 0 {
        quote! {}
    } else {
        quote! {
            #field_defs
            keys: [bytes::Bytes; #field_count],
            field_idx: usize,
            state: #state_name,
        }
    };

    (
        quote! {
            #vis struct #serializer_name {
                #struct_fields
            }
        },
        quote! {
            impl std::marker::Unpin for #serializer_name {}
        },
    )
}

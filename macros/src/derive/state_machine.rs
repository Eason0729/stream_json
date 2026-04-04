use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident, Visibility};

use super::super::attributes::{get_field_rename, get_variant_rename};
use super::super::ident::{serializer_name, state_name};
use super::shared::{
    generate_emit_key_arms, generate_emit_key_or_comma_arms, generate_emit_value_arms,
    generate_field_keys, generate_into_serializer_arm, generate_serializer_struct,
};

fn escaped_string_size(s: &str) -> usize {
    s.chars()
        .map(|c| match c {
            '"' | '\\' | '\n' | '\r' | '\t' => 2,
            c if c.is_control() => 6,
            c => c.len_utf8(),
        })
        .sum()
}

pub fn build_struct(name: &Ident, fields: &Fields, vis: &Visibility) -> TokenStream {
    let (field_infos, keys_array) = generate_field_keys(fields, name.clone());
    let field_count = field_infos.len();
    let state_name = state_name(name);
    let serializer_name = serializer_name(name);

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

    let size_body = if field_count == 0 {
        quote! { Some(2) }
    } else {
        let mut parts = Vec::new();
        for fi in &field_infos {
            let field_value = match fields {
                Fields::Named(_) => {
                    let ident = fi.ident.as_ref().expect("named field");
                    quote! { &self.#ident }
                }
                Fields::Unnamed(_) => {
                    let idx = syn::Index::from(fi.index);
                    quote! { &self.#idx }
                }
                Fields::Unit => quote! { &() },
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

    let into_serializer_arms: Vec<_> = variants
        .iter()
        .enumerate()
        .map(|(i, variant)| {
            let ident = &variant.ident;
            quote! {
                #name::#ident { .. } => #i
            }
        })
        .collect();

    let emit_arms: Vec<_> = variants
        .iter()
        .enumerate()
        .map(|(_i, variant)| {
            let ident = &variant.ident;

            let output = match &variant.fields {
                Fields::Unit => {
                    let variant_rename = get_variant_rename(variant);
                    if let Some(rename) = variant_rename {
                        let rename_str = format!("\"{}\"", rename);
                        quote! {
                            self.emit_pos = 0;
                            self.state = #state_name::ClosingBracket;
                            return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(#rename_str.as_bytes()))));
                        }
                    } else {
                        let variant_name = ident.to_string().to_lowercase();
                        quote! {
                            self.state = #state_name::ClosingBracket;
                            return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from(#variant_name))));
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    let count = fields.unnamed.len();
                    let variant_rename = get_variant_rename(variant);
                    if count == 0 {
                        if let Some(rename) = variant_rename {
                            let rename_str = format!("\"{}\"", rename);
                            quote! {
                                self.emit_pos = 0;
                                self.state = #state_name::ClosingBracket;
                                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(#rename_str.as_bytes()))));
                            }
                        } else {
                            quote! {
                                self.state = #state_name::ClosingBracket;
                                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"null"))));
                            }
                        }
                    } else {
                        let open_arm = if let Some(rename) = variant_rename {
                            let rename_str = format!("\"{}\"", rename);
                            quote! {
                                0 => {
                                    self.emit_pos = 1;
                                    return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(#rename_str.as_bytes()))));
                                }
                            }
                        } else {
                            quote! {
                                0 => {
                                    self.emit_pos = 1;
                                    return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"["))));
                                }
                            }
                        };
                        let mut field_arms = Vec::new();
                        for j in 1..=count {
                            field_arms.push(quote! {
                                #j => {
                                    self.emit_pos = #j + 1;
                                    return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"null"))));
                                }
                            });
                        }
                        let close_arm_pos = count + 1;
                        quote! {
                            match self.emit_pos {
                                #open_arm
                                #(#field_arms)*
                                #close_arm_pos => {
                                    self.emit_pos = 0;
                                    self.state = #state_name::ClosingBracket;
                                    return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"]"))));
                                }
                                _ => {
                                    self.emit_pos = 0;
                                    self.state = #state_name::ClosingBracket;
                                    return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"null"))));
                                }
                            }
                        }
                    }
                }
                Fields::Named(fields) => {
                    let count = fields.named.len();
                    let variant_rename = get_variant_rename(variant);
                    let keys: Vec<_> = if count == 1 {
                        if let Some(rename) = variant_rename.clone() {
                            vec![rename]
                        } else {
                            fields
                                .named
                                .iter()
                                .map(|f| {
                                    get_field_rename(f).unwrap_or_else(|| {
                                        f.ident.as_ref().expect("named field has ident").to_string()
                                    })
                                })
                                .collect()
                        }
                    } else {
                        fields
                            .named
                            .iter()
                            .map(|f| {
                                get_field_rename(f).unwrap_or_else(|| {
                                    f.ident.as_ref().expect("named field has ident").to_string()
                                })
                            })
                            .collect()
                    };
                    let key_strings: Vec<String> = keys.iter().map(|k| format!("\"{}\":", k)).collect();
                    if count == 0 {
                        if let Some(rename) = variant_rename {
                            let rename_str = format!("\"{}\"", rename);
                            quote! {
                                self.emit_pos = 0;
                                self.state = #state_name::ClosingBracket;
                                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(#rename_str.as_bytes()))));
                            }
                        } else {
                            quote! {
                                self.state = #state_name::ClosingBracket;
                                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"null"))));
                            }
                        }
                    } else {
                        let mut arms = Vec::new();
                        arms.push(quote! {
                            0 => {
                                self.emit_pos = 1;
                                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"{"))));
                            }
                        });
                        for (idx, _key) in keys.iter().enumerate() {
                            let key_str = &key_strings[idx];
                            let key_pos = idx * 3 + 1;
                            let val_pos = idx * 3 + 2;
                            let comma_pos = idx * 3 + 3;
                            arms.push(quote! {
                                #key_pos => {
                                    self.emit_pos = #val_pos;
                                    return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(#key_str.as_bytes()))));
                                }
                            });
                            arms.push(quote! {
                                #val_pos => {
                                    self.emit_pos = #comma_pos;
                                    return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"null"))));
                                }
                            });
                            if idx + 1 < count {
                                arms.push(quote! {
                                    #comma_pos => {
                                        self.emit_pos = #key_pos + 3;
                                        return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b","))));
                                    }
                                });
                            }
                        }
                        let closing_pos = count * 3;
                        arms.push(quote! {
                            #closing_pos => {
                                self.emit_pos = 0;
                                self.state = #state_name::ClosingBracket;
                                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"}"))));
                            }
                        });
                        quote! {
                            match self.emit_pos {
                                #(#arms)*
                                _ => {
                                    self.emit_pos = 0;
                                    self.state = #state_name::ClosingBracket;
                                    return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"null"))));
                                }
                            }
                        }
                    }
                }
            };

            quote! {
                #name::#ident { .. } => {
                    #output
                }
            }
        })
        .collect();

    let size_arms: Vec<_> = variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;
            match &variant.fields {
                Fields::Unit => {
                    let variant_rename = get_variant_rename(variant);
                    let size = if let Some(rename) = variant_rename {
                        rename.len() + 2
                    } else {
                        ident.to_string().to_lowercase().len()
                    };
                    quote! { #name::#ident => Some(#size), }
                }
                Fields::Unnamed(fields) => {
                    let count = fields.unnamed.len();
                    let variant_rename = get_variant_rename(variant);
                    let size = if count == 0 {
                        if let Some(rename) = variant_rename {
                            rename.len() + 2
                        } else {
                            4
                        }
                    } else if variant_rename.is_some() {
                        2 + count * 4 + 2
                    } else {
                        2 + count * 4
                    };
                    quote! { #name::#ident(..) => Some(#size), }
                }
                Fields::Named(fields) => {
                    let count = fields.named.len();
                    let variant_rename = get_variant_rename(variant);
                    let keys: Vec<_> = if count == 1 {
                        if let Some(rename) = variant_rename.clone() {
                            vec![rename]
                        } else {
                            fields
                                .named
                                .iter()
                                .map(|f| {
                                    get_field_rename(f).unwrap_or_else(|| {
                                        f.ident.as_ref().expect("named field has ident").to_string()
                                    })
                                })
                                .collect()
                        }
                    } else {
                        fields
                            .named
                            .iter()
                            .map(|f| {
                                get_field_rename(f).unwrap_or_else(|| {
                                    f.ident.as_ref().expect("named field has ident").to_string()
                                })
                            })
                            .collect()
                    };
                    let size = if count == 0 {
                        if let Some(rename) = variant_rename {
                            rename.len() + 2
                        } else {
                            4
                        }
                    } else {
                        let mut total = 4usize;
                        for (idx, key) in keys.iter().enumerate() {
                            if idx > 0 {
                                total += 1;
                            }
                            total += escaped_string_size(key) + 3 + 4;
                        }
                        total
                    };
                    quote! { #name::#ident { .. } => Some(#size), }
                }
            }
        })
        .collect();

    quote! {
        impl stream_json::serde::IntoSerializer for #name {
            type S = #serializer_name;
            fn into_serializer(self) -> Self::S {
                let variant_idx = match &self {
                    #(#into_serializer_arms,)*
                };
                #serializer_name {
                    inner: self,
                    variant_idx,
                    state: #state_name::Start,
                    emit_pos: 0,
                }
            }

            fn size(&self) -> Option<usize> {
                match self {
                    #(#size_arms)*
                }
            }
        }

        #vis struct #serializer_name {
            inner: #name,
            variant_idx: usize,
            state: #state_name,
            emit_pos: usize,
        }

        enum #state_name {
            Start,
            Emitting,
            ClosingBracket,
            Done,
        }

        impl stream_json::serde::Serializer for #serializer_name {
            fn poll(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Result<bytes::Bytes, stream_json::error::Error>>> {
                loop {
                    match &mut self.state {
                        #state_name::Start => {
                            self.state = #state_name::Emitting;
                            return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"["))));
                        }
                        #state_name::Emitting => {
                            match &mut self.inner {
                                #(#emit_arms)*
                            }
                        }
                        #state_name::ClosingBracket => {
                            self.state = #state_name::Done;
                            return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"]"))));
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

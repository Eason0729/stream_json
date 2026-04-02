use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident};

use super::super::attributes::{get_field_rename, get_variant_rename};
use super::super::ident::{serializer_name, state_name};
use super::shared::{
    generate_emit_key_arms, generate_emit_value_arms, generate_field_keys,
    generate_into_serializer_arm, generate_serializer_struct,
};

pub fn build_struct(name: &Ident, fields: &Fields) -> TokenStream {
    let (field_infos, keys_array) = generate_field_keys(fields, name.clone());
    let field_count = field_infos.len();

    if field_count == 0 {
        let serializer_name = serializer_name(name);
        return quote! {
            impl crate::serde::IntoSerializer for #name {
                type S = #serializer_name;
                fn into_serializer(self) -> Self::S {
                    #serializer_name { emitted: false }
                }
            }

            struct #serializer_name {
                emitted: bool,
            }

            impl crate::serde::Serializer for #serializer_name {
                fn poll(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Result<bytes::Bytes, crate::error::Error>>> {
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

    let into_serializer_arm = generate_into_serializer_arm(name, fields, &field_infos, &keys_array);
    let (serializer_struct, unpin_impl) = generate_serializer_struct(name, fields, &field_infos);

    let state_name = state_name(name);
    let serializer_name = serializer_name(name);

    let emit_key_arms = generate_emit_key_arms(name, &field_infos);
    let emit_value_arms = generate_emit_value_arms(name, &field_infos, field_count);

    let state_enum = if field_count == 0 {
        quote! {
            enum #state_name {
                Start,
                Done,
            }
        }
    } else {
        quote! {
            enum #state_name {
                Start,
                EmitKey,
                EmitValue,
                EmitComma,
                ClosingBrace,
                Done,
            }
        }
    };

    let poll_match_arms = if field_count == 0 {
        quote! {
            #state_name::Start => {
                self.state = #state_name::Done;
                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"{}"))));
            }
        }
    } else {
        quote! {
            #state_name::Start => {
                if #field_count == 0 {
                    self.state = #state_name::ClosingBrace;
                } else {
                    self.state = #state_name::EmitKey;
                }
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
            #state_name::EmitComma => {
                self.state = #state_name::EmitKey;
                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b","))));
            }
            #state_name::ClosingBrace => {
                self.state = #state_name::Done;
                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"}"))));
            }
        }
    };

    quote! {
        #into_serializer_arm

        #serializer_struct

        #state_enum

        impl crate::serde::Serializer for #serializer_name {
            fn poll(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Result<bytes::Bytes, crate::error::Error>>> {
                loop {
                    match &mut self.state {
                        #poll_match_arms
                        #state_name::Done => {
                            return std::task::Poll::Ready(None);
                        }
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

    quote! {
        impl crate::serde::IntoSerializer for #name {
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
        }

        struct #serializer_name {
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

        impl crate::serde::Serializer for #serializer_name {
            fn poll(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Result<bytes::Bytes, crate::error::Error>>> {
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

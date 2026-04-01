use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

#[proc_macro_derive(Serialize)]
pub fn derive_serialize(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as DeriveInput);
    let name = &input.ident;

    match &input.data {
        Data::Struct(data) => derive_struct(name, &data.fields),
        Data::Enum(data) => derive_enum(name, &data.variants),
        Data::Union(_) => TokenStream::from(
            syn::Error::new_spanned(input, "Union types are not supported").into_compile_error(),
        ),
    }
}

fn derive_struct(name: &syn::Ident, fields: &Fields) -> TokenStream {
    let serializer_name_str = format!("{}Serializer", name);
    let serializer_name = syn::Ident::new(&serializer_name_str, name.span());
    let state_name_str = format!("{}State", name);
    let state_name = syn::Ident::new(&state_name_str, name.span());

    let field_count = fields.len();

    let field_key_strings: Vec<String> = match fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .map(|f| {
                let key = f.ident.as_ref().expect("named field should have ident");
                format!("\"{}\":", key)
            })
            .collect(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, _)| format!("\"{}\":", i))
            .collect(),
        Fields::Unit => vec![],
    };

    let field_key_exprs: Vec<_> = field_key_strings
        .iter()
        .map(|s| quote! { bytes::Bytes::from(#s) })
        .collect();

    let field_serializers_field_defs: Vec<_> = (0..field_count)
        .map(|i| {
            let field_name = syn::Ident::new(&format!("f{}", i), name.span());
            quote! { #field_name: Box<dyn crate::serde::Serializer + Unpin> }
        })
        .collect();

    let field_serializers_from_self: Vec<_> = (0..field_count)
        .map(|i| {
            let field_name = syn::Ident::new(&format!("f{}", i), name.span());
            match fields {
                Fields::Named(fields) => {
                    let field = &fields.named[i];
                    let ident = &field.ident;
                    quote! { #field_name: Box::new(self.#ident.into_serializer()) }
                }
                Fields::Unnamed(_) => {
                    let idx = syn::Index::from(i);
                    quote! { #field_name: Box::new(self.#idx.into_serializer()) }
                }
                Fields::Unit => {
                    quote! { #field_name: Box::new(().into_serializer()) }
                }
            }
        })
        .collect();

    let emit_key_arms: Vec<_> = (0..field_count)
        .map(|i| {
            quote! {
                #i => {
                    let key = self.keys[#i].clone();
                    self.state = #state_name::EmitValue;
                    return std::task::Poll::Ready(Some(Ok(key)));
                }
            }
        })
        .collect();

    let emit_value_arms: Vec<_> = (0..field_count)
        .map(|i| {
            let field_name = syn::Ident::new(&format!("f{}", i), name.span());
            quote! {
                #i => {
                    let serializer = &mut self.#field_name;
                    match serializer.poll(cx) {
                        std::task::Poll::Ready(Some(result)) => {
                            return std::task::Poll::Ready(Some(result));
                        }
                        std::task::Poll::Ready(None) => {
                            if self.field_idx + 1 < #field_count {
                                self.field_idx += 1;
                                self.state = #state_name::EmitComma;
                            } else {
                                self.state = #state_name::ClosingBrace;
                            }
                            continue;
                        }
                        std::task::Poll::Pending => {
                            return std::task::Poll::Pending;
                        }
                    }
                }
            }
        })
        .collect();

    quote! {
        impl crate::serde::IntoSerializer for #name {
            type S = #serializer_name;
            fn into_serializer(self) -> Self::S {
                #serializer_name {
                    #(#field_serializers_from_self,)*
                    keys: [#(#field_key_exprs,)*],
                    field_idx: 0,
                    state: #state_name::Start,
                }
            }
        }

        struct #serializer_name {
            #(#field_serializers_field_defs,)*
            keys: [bytes::Bytes; #field_count],
            field_idx: usize,
            state: #state_name,
        }

        enum #state_name {
            Start,
            EmitKey,
            EmitValue,
            EmitComma,
            ClosingBrace,
            Done,
        }

        impl crate::serde::Serializer for #serializer_name {
            fn poll(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Result<bytes::Bytes, crate::error::Error>>> {
                loop {
                    match &mut self.state {
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
                        #state_name::Done => {
                            return std::task::Poll::Ready(None);
                        }
                    }
                }
            }
        }

        impl std::marker::Unpin for #serializer_name {}
    }
    .into()
}

fn derive_enum(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>,
) -> TokenStream {
    let serializer_name_str = format!("{}Serializer", name);
    let serializer_name = syn::Ident::new(&serializer_name_str, name.span());
    let state_name_str = format!("{}State", name);
    let state_name = syn::Ident::new(&state_name_str, name.span());

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
        .map(|(i, variant)| {
            let ident = &variant.ident;

            let output = match &variant.fields {
                Fields::Unit => {
                    quote! {
                        self.state = #state_name::ClosingBracket;
                        return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"null"))));
                    }
                }
                Fields::Unnamed(fields) => {
                    let count = fields.unnamed.len();
                    if count == 0 {
                        quote! {
                            self.state = #state_name::ClosingBracket;
                            return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"null"))));
                        }
                    } else {
                        let open_arm = quote! {
                            0 => {
                                self.emit_pos = 1;
                                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"["))));
                            }
                        };
                        let field_arms: Vec<_> = (1..=count)
                            .map(|j| {
                                quote! {
                                    #j => {
                                        self.emit_pos = #j + 1;
                                        return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"null"))));
                                    }
                                }
                            })
                            .collect();
                        let close_arm_pos = count + 1;
                        let close_arm = quote! {
                            #close_arm_pos => {
                                self.emit_pos = 0;
                                self.state = #state_name::ClosingBracket;
                                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"]"))));
                            }
                        };
                        quote! {
                            match self.emit_pos {
                                #open_arm
                                #(#field_arms)*
                                #close_arm
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
                    let keys: Vec<_> = fields.named.iter().map(|f| f.ident.as_ref().expect("named field should have ident")).collect();
                    let key_strings: Vec<String> = keys.iter().map(|k| format!("\"{}\":", k)).collect();
                    if count == 0 {
                        quote! {
                            self.state = #state_name::ClosingBracket;
                            return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"null"))));
                        }
                    } else {
                        let mut arms = Vec::new();
                        arms.push(quote! {
                            0 => {
                                self.emit_pos = 1;
                                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"{"))));
                            }
                        });
                        for (idx, key) in keys.iter().enumerate() {
                            let key_str = &key_strings[idx];
                            let key_pos = idx * 2 + 1;
                            let val_pos = idx * 2 + 2;
                            arms.push(quote! {
                                #key_pos => {
                                    self.emit_pos = #val_pos;
                                    return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(#key_str.as_bytes()))));
                                }
                            });
                            arms.push(quote! {
                                #val_pos => {
                                    self.emit_pos = #key_pos + 1;
                                    return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"null"))));
                                }
                            });
                        }
                        let closing_pos = count * 2;
                        arms.push(quote! {
                            #closing_pos => {
                                self.emit_pos = 0;
                                self.state = #state_name::ClosingBracket;
                                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"}"))));
                            }
                        });
                        arms.push(quote! {
                            _ => {
                                self.emit_pos = 0;
                                self.state = #state_name::ClosingBracket;
                                return std::task::Poll::Ready(Some(Ok(bytes::Bytes::from_static(b"null"))));
                            }
                        });
                        quote! {
                            match self.emit_pos {
                                #(#arms)*
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
                            continue;
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
    .into()
}

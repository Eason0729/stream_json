use quote::ToTokens;
use syn::{Attribute, Field, Meta, Variant};

pub fn get_stream_rename(attr: &Attribute) -> Option<String> {
    if !attr.path().is_ident("stream") {
        return None;
    }
    if let Meta::List(meta_list) = &attr.meta {
        let tokens = &meta_list.tokens;
        let mut iter = tokens.clone().into_iter().peekable();
        while let Some(token) = iter.next() {
            if let proc_macro2::TokenTree::Ident(ident) = &token {
                if ident == "rename" {
                    if let Some(next) = iter.next() {
                        if let proc_macro2::TokenTree::Punct(punct) = next {
                            if punct.as_char() == '=' {
                                if let Some(next) = iter.next() {
                                    if let proc_macro2::TokenTree::Literal(lit) = next {
                                        let lit_str = lit.to_string();
                                        if let Ok(expr) = syn::parse_str::<syn::Expr>(&lit_str) {
                                            if let syn::Expr::Lit(expr_lit) = expr {
                                                if let syn::Lit::Str(s) = expr_lit.lit {
                                                    return Some(s.value());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn get_field_rename(field: &Field) -> Option<String> {
    for attr in &field.attrs {
        if let Some(rename) = get_stream_rename(attr) {
            return Some(rename);
        }
    }
    None
}

pub fn get_variant_rename(variant: &Variant) -> Option<String> {
    for attr in &variant.attrs {
        if let Some(rename) = get_stream_rename(attr) {
            return Some(rename);
        }
    }
    None
}

pub fn get_skip_serialize_if(field: &Field) -> Option<proc_macro2::TokenStream> {
    for attr in &field.attrs {
        if !attr.path().is_ident("stream") {
            continue;
        }
        let s = attr.meta.to_token_stream().to_string();
        if let Some(pos) = s.find("skip_serialize_if") {
            let tail = &s[pos..];
            if let Some(start) = tail.find('"') {
                let tail = &tail[start + 1..];
                if let Some(end) = tail.find('"') {
                    let expr_src = &tail[..end];
                    if let Ok(tokens) = expr_src.parse::<proc_macro2::TokenStream>() {
                        return Some(tokens);
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_get_field_rename() {
        let field: Field = parse_quote! {
            #[stream(rename = "userName")]
            name: String
        };
        assert_eq!(get_field_rename(&field), Some("userName".to_string()));
    }

    #[test]
    fn test_get_field_rename_no_rename() {
        let field: Field = parse_quote! {
            name: String
        };
        assert_eq!(get_field_rename(&field), None);
    }

    #[test]
    fn test_get_variant_rename() {
        let variant: Variant = parse_quote! {
            #[stream(rename = "is_active")]
            Active
        };
        assert_eq!(get_variant_rename(&variant), Some("is_active".to_string()));
    }

    #[test]
    fn test_get_skip_serialize_if() {
        let field: Field = parse_quote! {
            #[stream(skip_serialize_if = "|v| v.is_empty()")]
            name: String
        };
        assert!(get_skip_serialize_if(&field).is_some());
    }
}

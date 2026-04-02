use quote::ToTokens;
use syn::{Attribute, Field, Variant};

pub fn get_stream_rename(attr: &Attribute) -> Option<String> {
    if !attr.path().is_ident("stream") {
        return None;
    }
    let mut rename = None;
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("rename") {
            let lit: syn::LitStr = meta.value()?.parse()?;
            rename = Some(lit.value());
        }
        Ok(())
    });
    rename
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

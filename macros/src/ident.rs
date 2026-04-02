use proc_macro2::Span;
use syn::Ident;

pub fn serializer_name(name: &Ident) -> Ident {
    Ident::new(&format!("{}Serializer", name), name.span())
}

pub fn state_name(name: &Ident) -> Ident {
    Ident::new(&format!("{}State", name), name.span())
}

pub fn field_name(index: usize, span: Span) -> Ident {
    Ident::new(&format!("f{}", index), span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serializer_name() {
        let name = syn::Ident::new("Person", Span::call_site());
        let ser_name = serializer_name(&name);
        assert_eq!(ser_name.to_string(), "PersonSerializer");
    }

    #[test]
    fn test_state_name() {
        let name = syn::Ident::new("Person", Span::call_site());
        let st_name = state_name(&name);
        assert_eq!(st_name.to_string(), "PersonState");
    }

    #[test]
    fn test_field_name() {
        let field = field_name(0, Span::call_site());
        assert_eq!(field.to_string(), "f0");
    }
}

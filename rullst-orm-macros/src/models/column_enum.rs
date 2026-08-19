use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_column_enum(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let normal_fields = &parsed.normal_fields;
    let column_enum_name = quote::format_ident!("{}Column", name);

    let column_variants: Vec<_> = normal_fields
        .iter()
        .map(|ident| {
            let name_str = ident.to_string();
            let mut chars = name_str.chars();
            let mut camel = match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            };
            camel = camel
                .split('_')
                .map(|s| {
                    let mut c = s.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    }
                })
                .collect();
            quote::format_ident!("{}", camel)
        })
        .collect();

    let column_to_string: Vec<_> = normal_fields
        .iter()
        .zip(column_variants.iter())
        .map(|(ident, variant)| {
            let field_name_str = ident.to_string();
            quote! { #column_enum_name::#variant => #field_name_str }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum #column_enum_name {
            #(#column_variants),*
        }
        impl #column_enum_name {
            pub fn as_str(&self) -> &'static str {
                match self {
                    #(#column_to_string),*
                }
            }
        }
    }
}

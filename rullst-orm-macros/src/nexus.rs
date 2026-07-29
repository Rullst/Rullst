use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

pub fn derive_nexus_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let table_name = format!("{}s", name.to_string().to_lowercase());
    let label = format!("{}s", name);

    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => {
                return syn::Error::new_spanned(
                    input,
                    "Nexus macro only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(input, "Nexus macro can only be used on structs")
                .to_compile_error()
                .into();
        }
    };

    let mut field_metas = Vec::new();
    let mut pk = "id".to_string();

    for field in fields {
        let field_ident = match &field.ident {
            Some(i) => i,
            None => continue,
        };
        let field_name_str = field_ident.to_string();

        let label_str = field_name_str.replace("_", " ");
        let mut label_chars = label_str.chars();
        let formatted_label = match label_chars.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + label_chars.as_str(),
        };

        let is_pk = field_name_str == "id";
        if is_pk {
            pk = field_name_str.clone();
        }

        // Infer FieldKind from syn::Type
        let ty = &field.ty;
        let mut type_str = quote!(#ty).to_string().replace(" ", "");

        // Handle Option<T>
        if type_str.starts_with("Option<") && type_str.ends_with(">") {
            type_str = type_str
                .trim_start_matches("Option<")
                .trim_end_matches(">")
                .to_string();
        }

        let field_kind = match type_str.as_str() {
            "String" => quote!(::rullst::nexus::FieldKind::Text),
            "bool" => quote!(::rullst::nexus::FieldKind::Boolean),
            "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize"
            | "f32" | "f64" => {
                quote!(::rullst::nexus::FieldKind::Number)
            }
            "chrono::DateTime<chrono::Utc>" | "DateTime<Utc>" => {
                quote!(::rullst::nexus::FieldKind::DateTime)
            }
            "chrono::NaiveDate" | "NaiveDate" => quote!(::rullst::nexus::FieldKind::Date),
            _ => {
                // If it's a custom type, assume Text for now. True enums would require deeper introspection.
                quote!(::rullst::nexus::FieldKind::Text)
            }
        };

        let hidden = is_pk || field_name_str == "password_hash" || field_name_str == "deleted_at";
        let readonly = is_pk || field_name_str == "created_at" || field_name_str == "updated_at";

        field_metas.push(quote! {
            ::rullst::nexus::FieldMeta {
                name: #field_name_str,
                label: #formatted_label,
                kind: #field_kind,
                hidden: #hidden,
                readonly: #readonly,
            }
        });
    }

    let expanded = quote! {
        impl ::rullst::nexus::NexusModel for #name {
            fn nexus_table() -> &'static str {
                #table_name
            }
            fn nexus_label() -> &'static str {
                #label
            }
            fn nexus_fields() -> Vec<::rullst::nexus::FieldMeta> {
                vec![
                    #(#field_metas),*
                ]
            }
            fn nexus_pk() -> &'static str {
                #pk
            }
        }
    };

    expanded.into()
}

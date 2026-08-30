use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitStr, Type, parse_macro_input};

#[derive(Default)]
struct ModelOptions {
    table: Option<String>,
    label: Option<String>,
    icon: Option<String>,
    primary_key: Option<String>,
}

#[derive(Default)]
struct FieldOptions {
    label: Option<String>,
    kind: Option<String>,
    options: Vec<String>,
    hidden: bool,
    readonly: bool,
    primary_key: bool,
}

fn parse_model_options(input: &DeriveInput) -> syn::Result<ModelOptions> {
    let mut options = ModelOptions::default();
    for attribute in &input.attrs {
        if !attribute.path().is_ident("nexus") && !attribute.path().is_ident("orm") {
            continue;
        }
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("table") {
                options.table = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("label") && attribute.path().is_ident("nexus") {
                options.label = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("icon") && attribute.path().is_ident("nexus") {
                options.icon = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("primary_key") && attribute.path().is_ident("nexus") {
                options.primary_key = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if attribute.path().is_ident("nexus") {
                return Err(meta.error("unsupported Nexus model option"));
            }
            Ok(())
        })?;
    }
    Ok(options)
}

fn parse_field_options(field: &syn::Field) -> syn::Result<FieldOptions> {
    let mut options = FieldOptions::default();
    for attribute in &field.attrs {
        if !attribute.path().is_ident("nexus") && !attribute.path().is_ident("orm") {
            continue;
        }
        attribute.parse_nested_meta(|meta| {
            let nexus_attribute = attribute.path().is_ident("nexus");
            if meta.path.is_ident("primary_key") {
                options.primary_key = true;
            } else if meta.path.is_ident("label") && nexus_attribute {
                options.label = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("kind") && nexus_attribute {
                options.kind = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("options") && nexus_attribute {
                let raw = meta.value()?.parse::<LitStr>()?.value();
                options.options = raw
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect();
            } else if meta.path.is_ident("hidden") && nexus_attribute {
                options.hidden = true;
            } else if meta.path.is_ident("readonly") && nexus_attribute {
                options.readonly = true;
            } else if nexus_attribute {
                return Err(meta.error("unsupported Nexus field option"));
            }
            Ok(())
        })?;
    }
    Ok(options)
}

fn unwrapped_type_name(field_type: &Type) -> String {
    let mut type_name = quote!(#field_type).to_string().replace(' ', "");
    if type_name.starts_with("Option<") && type_name.ends_with('>') {
        type_name = type_name[7..type_name.len() - 1].to_string();
    }
    type_name
}

fn inferred_field_kind(field_type: &Type) -> TokenStream2 {
    match unwrapped_type_name(field_type).as_str() {
        "String" | "&str" => quote!(::rullst::nexus::FieldKind::Text),
        "bool" => quote!(::rullst::nexus::FieldKind::Boolean),
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" | "f32"
        | "f64" => quote!(::rullst::nexus::FieldKind::Number),
        "chrono::DateTime<chrono::Utc>" | "DateTime<Utc>" => {
            quote!(::rullst::nexus::FieldKind::DateTime)
        }
        "chrono::NaiveDate" | "NaiveDate" => quote!(::rullst::nexus::FieldKind::Date),
        _ => quote!(::rullst::nexus::FieldKind::Text),
    }
}

fn configured_field_kind(field: &syn::Field, options: &FieldOptions) -> syn::Result<TokenStream2> {
    let Some(kind) = options.kind.as_deref() else {
        if options.options.is_empty() {
            return Ok(inferred_field_kind(&field.ty));
        }
        return enum_field_kind(field, &options.options);
    };

    let tokens = match kind {
        "text" => quote!(::rullst::nexus::FieldKind::Text),
        "textarea" => quote!(::rullst::nexus::FieldKind::Textarea),
        "email" => quote!(::rullst::nexus::FieldKind::Email),
        "url" => quote!(::rullst::nexus::FieldKind::Url),
        "number" => quote!(::rullst::nexus::FieldKind::Number),
        "boolean" => quote!(::rullst::nexus::FieldKind::Boolean),
        "date" => quote!(::rullst::nexus::FieldKind::Date),
        "datetime" => quote!(::rullst::nexus::FieldKind::DateTime),
        "password" => quote!(::rullst::nexus::FieldKind::Password),
        "json" => quote!(::rullst::nexus::FieldKind::Json),
        "enum" => return enum_field_kind(field, &options.options),
        unsupported => {
            return Err(syn::Error::new_spanned(
                field,
                format!("unsupported Nexus field kind `{unsupported}`"),
            ));
        }
    };
    if !options.options.is_empty() {
        return Err(syn::Error::new_spanned(
            field,
            "Nexus `options` can only be used with kind = \"enum\"",
        ));
    }
    Ok(tokens)
}

fn enum_field_kind(field: &syn::Field, options: &[String]) -> syn::Result<TokenStream2> {
    if options.is_empty() {
        return Err(syn::Error::new_spanned(
            field,
            "Nexus enum fields require a non-empty comma-separated `options` value",
        ));
    }
    Ok(quote!(::rullst::nexus::FieldKind::Enum {
        options: vec![#(#options),*]
    }))
}

fn humanize_field_name(field_name: &str) -> String {
    let label = field_name.replace('_', " ");
    let mut characters = label.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => String::new(),
    }
}

fn expand_nexus(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let model_options = parse_model_options(input)?;
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "Nexus macro only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "Nexus macro can only be used on structs",
            ));
        }
    };

    let table = model_options
        .table
        .unwrap_or_else(|| format!("{}s", name.to_string().to_lowercase()));
    let label = model_options.label.unwrap_or_else(|| format!("{name}s"));
    let icon = model_options.icon.unwrap_or_else(|| "📄".to_string());
    let configured_primary_key = model_options.primary_key;
    let mut inferred_primary_key = None;
    let mut field_metas = Vec::with_capacity(fields.len());

    for field in fields {
        let field_ident = field.ident.as_ref().ok_or_else(|| {
            syn::Error::new_spanned(field, "Nexus fields must have an identifier")
        })?;
        let raw_field_name = field_ident.to_string();
        let field_name = raw_field_name
            .strip_prefix("r#")
            .unwrap_or(&raw_field_name)
            .to_string();
        let options = parse_field_options(field)?;
        if options.primary_key || (configured_primary_key.is_none() && field_name == "id") {
            inferred_primary_key = Some(field_name.clone());
        }

        let kind = configured_field_kind(field, &options)?;
        let label = options
            .label
            .unwrap_or_else(|| humanize_field_name(&field_name));
        let is_primary_key = configured_primary_key.as_deref() == Some(field_name.as_str())
            || options.primary_key
            || (configured_primary_key.is_none() && field_name == "id");
        let hidden = options.hidden
            || is_primary_key
            || matches!(field_name.as_str(), "password_hash" | "deleted_at");
        let readonly = options.readonly
            || is_primary_key
            || matches!(field_name.as_str(), "created_at" | "updated_at");

        field_metas.push(quote! {
            ::rullst::nexus::FieldMeta {
                name: #field_name,
                label: #label,
                kind: #kind,
                hidden: #hidden,
                readonly: #readonly,
            }
        });
    }

    let primary_key = configured_primary_key
        .or(inferred_primary_key)
        .unwrap_or_else(|| "id".to_string());
    if !fields
        .iter()
        .filter_map(|field| field.ident.as_ref())
        .any(|field| field == primary_key.as_str())
    {
        return Err(syn::Error::new_spanned(
            input,
            format!("Nexus primary key `{primary_key}` is not a field on `{name}`"),
        ));
    }

    Ok(quote! {
        impl ::rullst::nexus::NexusModel for #name {
            fn nexus_table() -> &'static str { #table }
            fn nexus_label() -> &'static str { #label }
            fn nexus_icon() -> &'static str { #icon }
            fn nexus_fields() -> Vec<::rullst::nexus::FieldMeta> {
                vec![#(#field_metas),*]
            }
            fn nexus_pk() -> &'static str { #primary_key }
        }
    })
}

#[cfg_attr(mutants, mutants::skip)]
pub fn derive_nexus_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_nexus(&input) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn generates_typed_metadata_and_explicit_widgets() {
        let input: DeriveInput = parse_quote! {
            #[nexus(table = "articles", label = "Articles", icon = "📰")]
            struct Article {
                #[nexus(primary_key)]
                uuid: String,
                #[nexus(kind = "textarea", label = "Article body")]
                body: String,
                #[nexus(kind = "enum", options = "draft, published")]
                status: String,
                published: bool,
            }
        };
        let output = expand_nexus(&input)
            .expect("valid Nexus derive")
            .to_string();
        assert!(output.contains("fn nexus_table"));
        assert!(output.contains("\"articles\""));
        assert!(output.contains("FieldKind :: Textarea"));
        assert!(output.contains("FieldKind :: Enum"));
        assert!(output.contains("\"draft\""));
        assert!(output.contains("FieldKind :: Boolean"));
        assert!(output.contains("\"uuid\""));
    }

    #[test]
    fn rejects_invalid_widget_configuration_and_primary_key() {
        let invalid_kind: DeriveInput = parse_quote! {
            struct Article {
                id: i64,
                #[nexus(kind = "magic")]
                body: String,
            }
        };
        assert!(
            expand_nexus(&invalid_kind)
                .expect_err("invalid widget must fail")
                .to_string()
                .contains("unsupported Nexus field kind")
        );

        let invalid_pk: DeriveInput = parse_quote! {
            #[nexus(primary_key = "missing")]
            struct Article { id: i64 }
        };
        assert!(
            expand_nexus(&invalid_pk)
                .expect_err("missing primary key must fail")
                .to_string()
                .contains("is not a field")
        );
    }
}

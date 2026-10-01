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
    tenant_column: Option<String>,
}

#[derive(Default)]
struct FieldOptions {
    label: Option<String>,
    kind: Option<String>,
    options: Vec<String>,
    hidden: bool,
    readonly: bool,
    primary_key: bool,
    relation: bool,
    /// `#[orm(skip)]` / `#[sqlx(skip)]`: the field has no column.
    skipped: bool,
    orm_hidden: bool,
    masked: bool,
    encrypted: bool,
}

/// ORM relation declarations; such fields hold related models, not columns.
const ORM_RELATIONS: &[&str] = &[
    "has_many",
    "has_one",
    "belongs_to",
    "belongs_to_many",
    "morph_many",
    "morph_one",
    "morph_to",
];

/// Consumes the value of a shared `#[orm(...)]` option that Nexus does not
/// read, so `key = value` and `key(...)` options of `#[derive(Orm)]` parse.
pub(crate) fn skip_orm_option(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
    if meta.input.peek(syn::Token![=]) {
        meta.value()?.parse::<syn::Expr>()?;
    } else if meta.input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in meta.input);
        content.parse::<TokenStream2>()?;
    }
    Ok(())
}

fn parse_model_options(input: &DeriveInput) -> syn::Result<ModelOptions> {
    let mut options = ModelOptions::default();
    for attribute in &input.attrs {
        if !attribute.path().is_ident("nexus") && !attribute.path().is_ident("orm") {
            continue;
        }
        attribute.parse_nested_meta(|meta| {
            // `table_name` is the ORM's alias of `table`.
            if meta.path.is_ident("table")
                || (meta.path.is_ident("table_name") && attribute.path().is_ident("orm"))
            {
                options.table = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("label") && attribute.path().is_ident("nexus") {
                options.label = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("icon") && attribute.path().is_ident("nexus") {
                options.icon = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("primary_key") && attribute.path().is_ident("nexus") {
                options.primary_key = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("tenant") {
                options.tenant_column = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if attribute.path().is_ident("nexus") {
                return Err(meta.error("unsupported Nexus model option"));
            } else {
                skip_orm_option(&meta)?;
            }
            Ok(())
        })?;
    }
    Ok(options)
}

fn parse_field_options(field: &syn::Field) -> syn::Result<FieldOptions> {
    let mut options = FieldOptions::default();
    for attribute in &field.attrs {
        if attribute.path().is_ident("sqlx") {
            attribute.parse_nested_meta(|meta| {
                options.skipped |= meta.path.is_ident("skip");
                skip_orm_option(&meta)
            })?;
            continue;
        }
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
            } else if meta.path.is_ident("skip") {
                options.skipped = true;
            } else if meta.path.is_ident("hidden") {
                options.orm_hidden = true;
            } else if meta.path.is_ident("masked") {
                options.masked = true;
            } else if meta.path.is_ident("encrypted") {
                options.encrypted = true;
            } else {
                options.relation |= ORM_RELATIONS.iter().any(|key| meta.path.is_ident(key));
                skip_orm_option(&meta)?;
            }
            Ok(())
        })?;
    }
    Ok(options)
}

fn type_name(field_type: &Type) -> String {
    quote!(#field_type).to_string().replace(' ', "")
}

fn unwrapped_type_name(field_type: &Type) -> String {
    let mut type_name = type_name(field_type);
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

/// How the ORM's confidentiality markers constrain a field in Nexus.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Protection {
    None,
    /// `#[orm(masked)]`: a `Password` widget by default (never listed,
    /// searched, sorted or pre-filled); an explicit `kind` may relax it.
    Masked,
    /// `#[orm(hidden)]`: hidden and read-only by default; an explicit
    /// `kind = "password"` makes it a write-only `Password` field.
    Hidden,
    /// `#[orm(encrypted)]` or `SecretString`: the column holds an envelope that
    /// Nexus must never list, search or overwrite with plaintext.
    Sealed,
}

fn protection(field: &syn::Field, options: &FieldOptions) -> syn::Result<Protection> {
    let sealed = options.encrypted || crate::parser::is_secret_string_type(&field.ty);
    let explicit_widget = options.kind.is_some() || !options.options.is_empty();
    let password_widget = options.kind.as_deref() == Some("password") && options.options.is_empty();
    if sealed {
        if explicit_widget && !password_widget {
            return Err(syn::Error::new_spanned(
                field,
                "`#[orm(encrypted)]` and `SecretString` fields are hidden and read-only in Nexus; remove the #[nexus(kind/options)] override",
            ));
        }
        return Ok(Protection::Sealed);
    }
    if options.orm_hidden {
        if explicit_widget && !password_widget {
            return Err(syn::Error::new_spanned(
                field,
                "`#[orm(hidden)]` fields are hidden in Nexus; only #[nexus(kind = \"password\")] may expose them as a write-only field",
            ));
        }
        return Ok(Protection::Hidden);
    }
    if options.masked {
        return Ok(Protection::Masked);
    }
    Ok(Protection::None)
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
    let tenant_column = model_options.tenant_column;
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
        // Relations hold related models and skipped fields have no column.
        if options.relation || options.skipped {
            continue;
        }
        let protection = protection(field, &options)?;
        if tenant_column.as_deref() == Some(field_name.as_str())
            && (type_name(&field.ty) != "String"
                || options.kind.as_deref().is_some_and(|kind| kind != "text")
                || !options.options.is_empty())
        {
            return Err(syn::Error::new_spanned(
                field,
                "Nexus tenant columns must use a non-optional `String` with text metadata",
            ));
        }
        if options.primary_key || (configured_primary_key.is_none() && field_name == "id") {
            inferred_primary_key = Some(field_name.clone());
        }

        let password = quote!(::rullst::nexus::FieldKind::Password);
        let kind = match protection {
            Protection::Sealed | Protection::Hidden => password,
            Protection::Masked if options.kind.is_none() && options.options.is_empty() => password,
            Protection::Masked | Protection::None => configured_field_kind(field, &options)?,
        };
        // Sealed fields, and hidden ones without an explicit write-only
        // widget, are never listed, searched, rendered or written.
        let concealed = protection == Protection::Sealed
            || (protection == Protection::Hidden && options.kind.is_none());
        let label = options
            .label
            .unwrap_or_else(|| humanize_field_name(&field_name));
        let is_primary_key = configured_primary_key.as_deref() == Some(field_name.as_str())
            || options.primary_key
            || (configured_primary_key.is_none() && field_name == "id");
        let hidden = options.hidden
            || concealed
            || is_primary_key
            || matches!(field_name.as_str(), "password_hash" | "deleted_at");
        let readonly = options.readonly
            || concealed
            || is_primary_key
            || tenant_column.as_deref() == Some(field_name.as_str())
            || matches!(field_name.as_str(), "created_at" | "updated_at");
        let hidden = hidden || tenant_column.as_deref() == Some(field_name.as_str());

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
    if let Some(tenant_column) = tenant_column.as_deref()
        && !fields
            .iter()
            .filter_map(|field| field.ident.as_ref())
            .any(|field| field == tenant_column)
    {
        return Err(syn::Error::new_spanned(
            input,
            format!("Nexus tenant column `{tenant_column}` is not a field on `{name}`"),
        ));
    }

    let tenant_method = tenant_column
        .as_deref()
        .map(|column| quote!(fn nexus_tenant_column() -> Option<&'static str> { Some(#column) }));

    Ok(quote! {
        impl ::rullst::nexus::NexusModel for #name {
            fn nexus_table() -> &'static str { #table }
            fn nexus_label() -> &'static str { #label }
            fn nexus_icon() -> &'static str { #icon }
            fn nexus_fields() -> Vec<::rullst::nexus::FieldMeta> {
                vec![#(#field_metas),*]
            }
            fn nexus_pk() -> &'static str { #primary_key }
            #tenant_method
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
mod tests;

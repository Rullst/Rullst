use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeSet;
use syn::{Data, DeriveInput, Fields, LitStr};

const MAX_VARIANTS: usize = 64;
const MAX_LABEL_BYTES: usize = 63;

#[derive(Clone, Copy)]
enum RenameAll {
    Lowercase,
    SnakeCase,
}

#[derive(Default)]
struct EnumAttributes {
    type_name: Option<String>,
    rename_all: Option<RenameAll>,
}

pub fn derive_enum_impl(input: TokenStream) -> TokenStream {
    match expand_enum(input) {
        Ok(expanded) => expanded,
        Err(error) => error.to_compile_error(),
    }
}

fn expand_enum(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<DeriveInput>(input)?;
    let name = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "Enum macro does not support generic database enums",
        ));
    }
    let variants = match &input.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "Enum macro can only be used on enums",
            ));
        }
    };
    if variants.is_empty() || variants.len() > MAX_VARIANTS {
        return Err(syn::Error::new_spanned(
            variants,
            format!("Enum macro requires 1-{MAX_VARIANTS} unit variants"),
        ));
    }

    let attributes = parse_enum_attributes(&input)?;
    let type_name = attributes
        .type_name
        .unwrap_or_else(|| to_snake_case(&name.to_string()));
    validate_type_name(&type_name, name)?;

    let mut variant_idents = Vec::with_capacity(variants.len());
    let mut labels = Vec::with_capacity(variants.len());
    let mut seen = BTreeSet::new();
    for variant in variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                "Enum macro supports only unit variants",
            ));
        }
        let rename = parse_variant_rename(variant)?;
        let original = variant.ident.to_string();
        let label = rename.unwrap_or_else(|| match attributes.rename_all {
            Some(RenameAll::Lowercase) => original.to_ascii_lowercase(),
            Some(RenameAll::SnakeCase) => to_snake_case(&original),
            None => original,
        });
        validate_label(&label, variant)?;
        if !seen.insert(label.clone()) {
            return Err(syn::Error::new_spanned(
                variant,
                format!("duplicate database enum label `{label}`"),
            ));
        }
        variant_idents.push(&variant.ident);
        labels.push(label);
    }

    let display_arms = variant_idents
        .iter()
        .zip(labels.iter())
        .map(|(variant, label)| quote! { #name::#variant => #label });
    let parse_arms = variant_idents
        .iter()
        .zip(labels.iter())
        .map(|(variant, label)| quote! { #label => Ok(#name::#variant) });
    let decode_arms = variant_idents
        .iter()
        .zip(labels.iter())
        .map(|(variant, label)| quote! { #label => Ok(#name::#variant) });

    Ok(quote! {
        impl rullst_orm::DatabaseEnum for #name {
            const TYPE_NAME: &'static str = #type_name;
            const VARIANTS: &'static [&'static str] = &[#(#labels),*];
        }

        impl ::std::fmt::Display for #name {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                let value = match self {
                    #(#display_arms,)*
                };
                formatter.write_str(value)
            }
        }

        impl ::std::str::FromStr for #name {
            type Err = ::std::string::String;

            fn from_str(value: &str) -> ::std::result::Result<Self, Self::Err> {
                match value {
                    #(#parse_arms,)*
                    _ => Err(::std::format!(
                        "Invalid value for enum {}: {}",
                        stringify!(#name),
                        value
                    )),
                }
            }
        }

        impl From<#name> for rullst_orm::RullstValue {
            fn from(value: #name) -> Self {
                rullst_orm::RullstValue::String(value.to_string())
            }
        }

        impl TryFrom<rullst_orm::RullstValue> for #name {
            type Error = rullst_orm::Error;

            fn try_from(value: rullst_orm::RullstValue) -> Result<Self, Self::Error> {
                let value: String = value.try_into().map_err(|_| {
                    rullst_orm::Error::Internal("Enum value must be a string".to_string())
                })?;
                value.parse().map_err(|error: String| rullst_orm::Error::Internal(error))
            }
        }

        impl rullst_orm::_serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: rullst_orm::_serde::Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }

        impl<'de> rullst_orm::_serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: rullst_orm::_serde::Deserializer<'de>,
            {
                let value = <String as rullst_orm::_serde::Deserialize<'de>>::deserialize(deserializer)?;
                value.parse().map_err(rullst_orm::_serde::de::Error::custom)
            }
        }

        impl<'query, DB> rullst_orm::_sqlx::Encode<'query, DB> for #name
        where
            DB: rullst_orm::_sqlx::Database,
            &'query str: rullst_orm::_sqlx::Encode<'query, DB>,
        {
            fn encode_by_ref(
                &self,
                buffer: &mut <DB as rullst_orm::_sqlx::Database>::ArgumentBuffer,
            ) -> Result<rullst_orm::_sqlx::encode::IsNull, rullst_orm::_sqlx::error::BoxDynError> {
                let value = match self {
                    #(#display_arms,)*
                };
                <&str as rullst_orm::_sqlx::Encode<'query, DB>>::encode(value, buffer)
            }

            fn size_hint(&self) -> usize {
                let value = match self {
                    #(#display_arms,)*
                };
                <&str as rullst_orm::_sqlx::Encode<'query, DB>>::size_hint(&value)
            }
        }

        impl<'row, DB> rullst_orm::_sqlx::Decode<'row, DB> for #name
        where
            DB: rullst_orm::_sqlx::Database,
            &'row str: rullst_orm::_sqlx::Decode<'row, DB>,
        {
            fn decode(
                value: <DB as rullst_orm::_sqlx::Database>::ValueRef<'row>,
            ) -> Result<Self, rullst_orm::_sqlx::error::BoxDynError> {
                let value = <&'row str as rullst_orm::_sqlx::Decode<'row, DB>>::decode(value)?;
                match value {
                    #(#decode_arms,)*
                    _ => Err(::std::format!(
                        "invalid value {:?} for enum {}",
                        value,
                        stringify!(#name)
                    ).into()),
                }
            }
        }

        impl rullst_orm::_sqlx::Type<rullst_orm::_sqlx::Any> for #name {
            fn type_info() -> rullst_orm::_sqlx::any::AnyTypeInfo {
                <str as rullst_orm::_sqlx::Type<rullst_orm::_sqlx::Any>>::type_info()
            }
        }

        impl rullst_orm::_sqlx::Type<rullst_orm::_sqlx::Sqlite> for #name {
            fn type_info() -> rullst_orm::_sqlx::sqlite::SqliteTypeInfo {
                <str as rullst_orm::_sqlx::Type<rullst_orm::_sqlx::Sqlite>>::type_info()
            }
        }

        impl rullst_orm::_sqlx::Type<rullst_orm::_sqlx::MySql> for #name {
            fn type_info() -> rullst_orm::_sqlx::mysql::MySqlTypeInfo {
                rullst_orm::_sqlx::mysql::MySqlTypeInfo::__enum()
            }
        }

        impl rullst_orm::_sqlx::Type<rullst_orm::_sqlx::Postgres> for #name {
            fn type_info() -> rullst_orm::_sqlx::postgres::PgTypeInfo {
                rullst_orm::_sqlx::postgres::PgTypeInfo::with_name(#type_name)
            }
        }

        impl rullst_orm::_sqlx::postgres::PgHasArrayType for #name {
            fn array_type_info() -> rullst_orm::_sqlx::postgres::PgTypeInfo {
                rullst_orm::_sqlx::postgres::PgTypeInfo::array_of(#type_name)
            }
        }
    })
}

fn parse_enum_attributes(input: &DeriveInput) -> syn::Result<EnumAttributes> {
    let mut parsed = EnumAttributes::default();
    for attribute in input
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("rullst_enum"))
    {
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("type_name") {
                if parsed.type_name.is_some() {
                    return Err(meta.error("duplicate `type_name`"));
                }
                parsed.type_name = Some(meta.value()?.parse::<LitStr>()?.value());
                return Ok(());
            }
            if meta.path.is_ident("rename_all") {
                if parsed.rename_all.is_some() {
                    return Err(meta.error("duplicate `rename_all`"));
                }
                let value = meta.value()?.parse::<LitStr>()?.value();
                parsed.rename_all = Some(match value.as_str() {
                    "lowercase" => RenameAll::Lowercase,
                    "snake_case" => RenameAll::SnakeCase,
                    _ => return Err(meta.error("rename_all must be `lowercase` or `snake_case`")),
                });
                return Ok(());
            }
            Err(meta.error("unsupported rullst_enum attribute"))
        })?;
    }
    Ok(parsed)
}

fn parse_variant_rename(variant: &syn::Variant) -> syn::Result<Option<String>> {
    let mut rename = None;
    for attribute in variant
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("rullst_enum"))
    {
        attribute.parse_nested_meta(|meta| {
            if !meta.path.is_ident("rename") {
                return Err(meta.error("enum variants support only `rename`"));
            }
            if rename.is_some() {
                return Err(meta.error("duplicate `rename`"));
            }
            rename = Some(meta.value()?.parse::<LitStr>()?.value());
            Ok(())
        })?;
    }
    Ok(rename)
}

fn validate_type_name(value: &str, span: impl quote::ToTokens) -> syn::Result<()> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > MAX_LABEL_BYTES
        || !(bytes[0].is_ascii_alphabetic() || bytes[0] == b'_')
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        return Err(syn::Error::new_spanned(
            span,
            "database enum type_name must be a 1-63 byte unqualified ASCII identifier",
        ));
    }
    Ok(())
}

fn validate_label(value: &str, span: impl quote::ToTokens) -> syn::Result<()> {
    if value.is_empty()
        || value.len() > MAX_LABEL_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
    {
        return Err(syn::Error::new_spanned(
            span,
            "database enum labels must be 1-63 printable ASCII bytes",
        ));
    }
    Ok(())
}

fn to_snake_case(value: &str) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len() + 4);
    for (index, character) in characters.iter().copied().enumerate() {
        if character.is_ascii_uppercase() {
            let previous_is_lower_or_digit = index > 0
                && (characters[index - 1].is_ascii_lowercase()
                    || characters[index - 1].is_ascii_digit());
            let next_is_lower = characters
                .get(index + 1)
                .is_some_and(char::is_ascii_lowercase);
            if index > 0 && (previous_is_lower_or_digit || next_is_lower) {
                output.push('_');
            }
            output.push(character.to_ascii_lowercase());
        } else {
            output.push(character);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn derives_closed_metadata_and_database_codecs() {
        let output = derive_enum_impl(quote! {
            #[rullst_enum(type_name = "account_status", rename_all = "snake_case")]
            pub enum Status {
                AwaitingReview,
                #[rullst_enum(rename = "live")]
                Active,
            }
        });
        let output = output.to_string();
        assert!(output.contains("DatabaseEnum for Status"));
        assert!(output.contains("account_status"));
        assert!(output.contains("awaiting_review"));
        assert!(output.contains("PgHasArrayType for Status"));
        assert!(output.contains("Type < rullst_orm :: _sqlx :: MySql > for Status"));
    }

    #[test]
    fn rejects_non_enum_and_data_variants() {
        let not_enum = derive_enum_impl(quote! { pub struct NotEnum { id: i32 } }).to_string();
        assert!(not_enum.contains("Enum macro can only be used on enums"));

        let data_variant = derive_enum_impl(quote! {
            pub enum Status { Active(String) }
        })
        .to_string();
        assert!(data_variant.contains("supports only unit variants"));
    }

    #[test]
    fn case_conversion_handles_words_and_acronyms() {
        assert_eq!(to_snake_case("AwaitingReview"), "awaiting_review");
        assert_eq!(to_snake_case("HTTPStatus"), "http_status");
    }
}

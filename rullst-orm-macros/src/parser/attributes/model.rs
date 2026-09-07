use super::common::*;
use crate::parser::SoftDeleteConfig;
use std::collections::HashSet;
use syn::meta::ParseNestedMeta;
use syn::{DeriveInput, Token, spanned::Spanned};

pub(in crate::parser) struct ModelAttributes {
    pub backend: String,
    pub table_name: String,
    pub global_scope: String,
    pub tenant_column: String,
    pub auditable: bool,
    pub searchable: bool,
    pub policy: String,
    pub before_save: String,
    pub after_save: String,
    pub before_delete: String,
    pub after_delete: String,
    pub after_fetch: String,
    pub soft_delete: Option<SoftDeleteConfig>,
}

impl ModelAttributes {
    pub fn parse(input: &DeriveInput) -> Result<Self, syn::Error> {
        let table_name = format!("{}s", input.ident.to_string().to_lowercase());
        validate_sql_identifier(&table_name, "default table name", input.ident.span())?;
        let mut parsed = Self::new(table_name);
        let mut seen = HashSet::new();
        for attribute in input
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident("orm"))
        {
            attribute.parse_nested_meta(|meta| parsed.apply(meta, &mut seen))?;
        }
        Ok(parsed)
    }

    fn new(table_name: String) -> Self {
        Self {
            backend: "sqlx".to_string(),
            table_name,
            global_scope: String::new(),
            tenant_column: String::new(),
            auditable: false,
            searchable: false,
            policy: String::new(),
            before_save: String::new(),
            after_save: String::new(),
            before_delete: String::new(),
            after_delete: String::new(),
            after_fetch: String::new(),
            soft_delete: None,
        }
    }

    fn apply(
        &mut self,
        meta: ParseNestedMeta<'_>,
        seen: &mut HashSet<&'static str>,
    ) -> Result<(), syn::Error> {
        if meta.path.is_ident("auditable") {
            mark_once(seen, "auditable", &meta)?;
            self.auditable = true;
            return Ok(());
        }
        if meta.path.is_ident("searchable") {
            mark_once(seen, "searchable", &meta)?;
            self.searchable = true;
            return Ok(());
        }
        if meta.path.is_ident("soft_delete") {
            mark_once(seen, "soft_delete", &meta)?;
            self.soft_delete = Some(parse_soft_delete(&meta)?);
            return Ok(());
        }

        let key = path_name(&meta)?;
        match key.as_str() {
            "backend" => {
                mark_once(seen, "backend", &meta)?;
                let value = string_value(&meta)?;
                if !matches!(value.as_str(), "sqlx" | "turso") {
                    return Err(meta.error("ORM backend must be `sqlx` or `turso`"));
                }
                self.backend = value;
            }
            "table" | "table_name" => {
                mark_once(seen, "table", &meta)?;
                let value = string_value(&meta)?;
                validate_sql_identifier(&value, "table name", meta.path.span())?;
                self.table_name = value;
            }
            "tabel" | "tbl" | "tablename" => {
                return Err(
                    meta.error("unknown model attribute; did you mean `#[orm(table = \"...\")]`?")
                );
            }
            // Nexus reads this option from the shared `orm` namespace. The ORM
            // validates its shape but deliberately assigns it no SQL meaning.
            "tenant" => {
                mark_once(seen, "nexus_tenant", &meta)?;
                let value = string_value(&meta)?;
                validate_rust_identifier(&value, "Nexus tenant field", meta.path.span())?;
            }
            "global_scope" => {
                self.global_scope = identifier_value(&meta, seen, "global_scope")?;
            }
            "tenant_column" => {
                self.tenant_column = identifier_value(&meta, seen, "tenant_column")?;
            }
            "policy" => self.policy = identifier_value(&meta, seen, "policy")?,
            "before_save" => {
                self.before_save = identifier_value(&meta, seen, "before_save")?;
            }
            "after_save" => self.after_save = identifier_value(&meta, seen, "after_save")?,
            "before_delete" => {
                self.before_delete = identifier_value(&meta, seen, "before_delete")?;
            }
            "after_delete" => {
                self.after_delete = identifier_value(&meta, seen, "after_delete")?;
            }
            "after_fetch" => self.after_fetch = identifier_value(&meta, seen, "after_fetch")?,
            _ => return Err(meta.error(format!("unsupported ORM model option `{key}`"))),
        }
        Ok(())
    }
}

fn parse_soft_delete(meta: &ParseNestedMeta<'_>) -> Result<SoftDeleteConfig, syn::Error> {
    let mut column = None;
    let mut value = None;
    let mut delval = None;
    let mut seen = HashSet::new();
    if meta.input.peek(syn::token::Paren) {
        meta.parse_nested_meta(|nested| {
            let key = path_name(&nested)?;
            match key.as_str() {
                "field" | "column" => {
                    mark_once(&mut seen, "column", &nested)?;
                    let parsed = string_value(&nested)?;
                    validate_sql_identifier(&parsed, "soft-delete column", nested.path.span())?;
                    column = Some(parsed);
                }
                "value" => {
                    mark_once(&mut seen, "value", &nested)?;
                    value = Some(bounded_sql_fragment(&nested)?);
                }
                "delval" => {
                    mark_once(&mut seen, "delval", &nested)?;
                    delval = Some(bounded_sql_fragment(&nested)?);
                }
                _ => return Err(nested.error(format!("unsupported soft_delete option `{key}`"))),
            }
            Ok(())
        })?;
    } else if meta.input.peek(Token![=]) {
        return Err(meta.error("soft_delete accepts no value or a parenthesized option list"));
    }
    Ok(SoftDeleteConfig {
        column: column.unwrap_or_else(|| "deleted_at".to_string()),
        value: value.unwrap_or_default(),
        delval: delval.unwrap_or_default(),
    })
}

fn bounded_sql_fragment(meta: &ParseNestedMeta<'_>) -> Result<String, syn::Error> {
    let value = string_value(meta)?;
    if value.len() > 128
        || value.contains([';', '\0', '#'])
        || value.contains("--")
        || value.contains("/*")
        || value.contains("*/")
    {
        return Err(meta.error(
            "soft-delete SQL fragments are limited to 128 bytes and cannot contain separators or comments",
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    #[test]
    fn persisted_column_names_must_match_the_portable_sql_grammar() {
        for column in ["r#type".to_string(), "café".to_string(), "a".repeat(65)] {
            let identifier =
                syn::parse_str::<syn::Ident>(&column).expect("valid Rust field identifier");
            let input: syn::DeriveInput = syn::parse_quote! {
                struct Record { id: i32, #identifier: String }
            };
            assert!(
                crate::parser::parse(&input).is_err(),
                "accepted non-portable column: {column}"
            );
        }
    }

    #[test]
    fn soft_delete_fragments_reject_every_supported_dialect_comment() {
        for fragment in [
            "0 /*",
            "0 /* hidden */",
            "0 */",
            "0 # trailing",
            "0 -- trailing",
        ] {
            for option in ["value", "delval"] {
                let option = syn::Ident::new(option, proc_macro2::Span::call_site());
                let input: syn::DeriveInput = syn::parse_quote! {
                    #[orm(soft_delete(#option = #fragment))]
                    struct Record { id: i32, deleted_at: Option<String> }
                };
                assert!(
                    crate::parser::parse(&input).is_err(),
                    "accepted SQL comment: {fragment}"
                );
            }
        }
        for fragment in [
            "0",
            "NULL",
            "CURRENT_TIMESTAMP",
            "strftime('%Y-%m-%d', 'now')",
        ] {
            let input: syn::DeriveInput = syn::parse_quote! {
                #[orm(soft_delete(delval = #fragment))]
                struct Record { id: i32, deleted_at: Option<String> }
            };
            assert!(
                crate::parser::parse(&input).is_ok(),
                "rejected bounded expression: {fragment}"
            );
        }
    }
}

//! Static ORM model extraction shared by `make:migration:auto` and
//! `inspect schema`. It mirrors `rullst_orm::schema_diff`; keep the two parsers
//! in step (the CLI does not depend on `rullst-orm`).

use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedField {
    pub name: String,
    pub rust_type: String,
    pub is_option: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedTable {
    pub table_name: String,
    pub struct_name: String,
    pub fields: Vec<ParsedField>,
}

pub fn extract_tables_from_ast() -> Vec<ParsedTable> {
    extract_tables_from_root("src")
}

fn extract_tables_from_root(root: impl AsRef<Path>) -> Vec<ParsedTable> {
    let mut tables = Vec::new();
    let walker = WalkDir::new(root).into_iter().filter_map(|e| e.ok());

    for entry in walker {
        if entry.path().extension().is_some_and(|ext| ext == "rs") {
            let content = match fs::read_to_string(entry.path()) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if let Ok(ast) = syn::parse_file(&content) {
                for item in ast.items {
                    if let syn::Item::Struct(item_struct) = item {
                        let mut has_orm_derive = false;
                        let mut table_name = None;

                        for attr in &item_struct.attrs {
                            if attr.path().is_ident("derive") {
                                let _ = attr.parse_nested_meta(|meta| {
                                    // `Orm`, `rullst_orm::Orm` or another path to it.
                                    if meta
                                        .path
                                        .segments
                                        .last()
                                        .is_some_and(|last| last.ident == "Orm")
                                    {
                                        has_orm_derive = true;
                                    }
                                    Ok(())
                                });
                            }
                            if attr.path().is_ident("orm") {
                                let _ = attr.parse_nested_meta(|meta| {
                                    if (meta.path.is_ident("table")
                                        || meta.path.is_ident("table_name"))
                                        && meta.input.peek(syn::Token![=])
                                    {
                                        table_name =
                                            Some(meta.value()?.parse::<syn::LitStr>()?.value());
                                        return Ok(());
                                    }
                                    skip_meta_value(&meta)
                                });
                            }
                        }

                        if has_orm_derive {
                            let struct_name = item_struct.ident.to_string();
                            let t_name = table_name.unwrap_or_else(|| {
                                let snake = struct_name.to_lowercase();
                                format!("{}s", snake)
                            });

                            let mut fields = Vec::new();
                            if let syn::Fields::Named(named_fields) = item_struct.fields {
                                for field in named_fields.named {
                                    if let Some(ident) = field.ident {
                                        let field_name = ident.to_string();
                                        let (rust_type, is_option) = extract_type_name(&field.ty);

                                        let mut skip = false;
                                        for f_attr in &field.attrs {
                                            if f_attr.path().is_ident("sqlx")
                                                || f_attr.path().is_ident("orm")
                                            {
                                                let _ = f_attr.parse_nested_meta(|meta| {
                                                    if meta.path.is_ident("skip") {
                                                        skip = true;
                                                    }
                                                    skip_meta_value(&meta)
                                                });
                                            }
                                        }
                                        if skip {
                                            continue;
                                        }

                                        fields.push(ParsedField {
                                            name: field_name,
                                            rust_type,
                                            is_option,
                                        });
                                    }
                                }
                            }

                            tables.push(ParsedTable {
                                table_name: t_name,
                                struct_name,
                                fields,
                            });
                        }
                    }
                }
            }
        }
    }
    tables
}

/// Consumes the value of an option this extractor does not read, so the
/// options after it (for example `table` after `tenant_column = "..."`) are
/// still visited instead of aborting the attribute at the first value.
fn skip_meta_value(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
    if meta.input.peek(syn::Token![=]) {
        meta.value()?.parse::<syn::Expr>()?;
    } else if meta.input.peek(syn::token::Paren) {
        meta.parse_nested_meta(|nested| skip_meta_value(&nested))?;
    }
    Ok(())
}

#[cfg_attr(mutants, mutants::skip)]
fn extract_type_name(ty: &syn::Type) -> (String, bool) {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        let type_name = segment.ident.to_string();
        if type_name == "Option"
            && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
        {
            let (inner_name, _) = extract_type_name(inner_ty);
            return (inner_name, true);
        }
        return (type_name, false);
    }
    ("Unknown".to_string(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tables_in(source: &str) -> Vec<ParsedTable> {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("models.rs"), source).unwrap();
        let mut tables = extract_tables_from_root(directory.path());
        tables.sort_by(|left, right| left.table_name.cmp(&right.table_name));
        tables
    }

    #[test]
    fn explicit_default_optional_and_skipped_fields_are_extracted() {
        let tables = tables_in(
            r#"
                #[derive(Debug, Orm)]
                #[orm(table = "people")]
                struct Person {
                    id: i64,
                    name: Option<String>,
                    #[orm(skip)]
                    transient: String,
                    #[sqlx(skip)]
                    computed: i32,
                }

                #[derive(Orm)]
                struct BlogPost { id: u64, payload: Vec<u8> }

                #[derive(Debug)]
                struct NotAModel { id: i64 }
            "#,
        );
        let names: Vec<_> = tables.iter().map(|t| t.table_name.as_str()).collect();
        assert_eq!(names, ["blogposts", "people"]);
        assert_eq!(tables[0].fields[1].rust_type, "Vec");
        assert_eq!(
            tables[1].fields,
            [
                ParsedField {
                    name: "id".to_string(),
                    rust_type: "i64".to_string(),
                    is_option: false,
                },
                ParsedField {
                    name: "name".to_string(),
                    rust_type: "String".to_string(),
                    is_option: true,
                },
            ]
        );
    }

    #[test]
    fn table_options_after_other_values_and_qualified_derives_are_extracted() {
        let tables = tables_in(
            r#"
                #[derive(FromRow, Orm)]
                #[orm(tenant_column = "account_id", soft_delete(column = "removed_at"), table = "user_projects")]
                struct Project {
                    id: i64,
                    account_id: String,
                    #[orm(encrypted, skip)]
                    cached: String,
                }

                #[derive(rullst_orm::Orm)]
                #[orm(table_name = "audit_events", auditable)]
                struct AuditEvent { id: i64 }
            "#,
        );
        let names: Vec<_> = tables.iter().map(|t| t.table_name.as_str()).collect();
        assert_eq!(names, ["audit_events", "user_projects"]);
        assert_eq!(tables[1].struct_name, "Project");
        assert_eq!(
            tables[1].fields.len(),
            2,
            "the skipped field must be omitted"
        );
    }
}

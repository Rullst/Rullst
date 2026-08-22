use std::fs;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct ParsedField {
    pub name: String,
    pub rust_type: String,
    pub is_option: bool,
}

#[derive(Debug)]
pub struct ParsedTable {
    pub table_name: String,
    pub struct_name: String,
    pub fields: Vec<ParsedField>,
}

#[cfg_attr(mutants, mutants::skip)]
pub fn extract_tables_from_ast() -> Vec<ParsedTable> {
    let mut tables = Vec::new();
    let walker = WalkDir::new("src").into_iter().filter_map(|e| e.ok());

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
                                    if meta.path.is_ident("Orm") {
                                        has_orm_derive = true;
                                    }
                                    Ok(())
                                });
                            }
                            if attr.path().is_ident("orm") {
                                let _ = attr.parse_nested_meta(|meta| {
                                    if meta.path.is_ident("table")
                                        && let Ok(value) = meta.value()
                                        && let Ok(lit) = value.parse::<syn::LitStr>()
                                    {
                                        table_name = Some(lit.value());
                                    }
                                    Ok(())
                                });
                            }
                        }

                        if has_orm_derive {
                            let struct_name = item_struct.ident.to_string();
                            let t_name = table_name.unwrap_or_else(|| {
                                // Default table name: snake_case and pluralized (simple logic)
                                let snake = struct_name.to_lowercase();
                                format!("{}s", snake) // VERY rudimentary default
                            });

                            let mut fields = Vec::new();
                            if let syn::Fields::Named(named_fields) = item_struct.fields {
                                for field in named_fields.named {
                                    if let Some(ident) = field.ident {
                                        let field_name = ident.to_string();
                                        let (rust_type, is_option) = extract_type_name(&field.ty);

                                        // Skip fields with #[sqlx(skip)] or #[orm(skip)]
                                        let mut skip = false;
                                        for f_attr in &field.attrs {
                                            if f_attr.path().is_ident("sqlx")
                                                || f_attr.path().is_ident("orm")
                                            {
                                                let _ = f_attr.parse_nested_meta(|meta| {
                                                    if meta.path.is_ident("skip") {
                                                        skip = true;
                                                    }
                                                    Ok(())
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

    #[test]
    fn test_extract_type_name() {
        let ty_i32: syn::Type = syn::parse_str("i32").unwrap();
        let (name, is_opt) = extract_type_name(&ty_i32);
        assert_eq!(name, "i32");
        assert!(!is_opt);

        let ty_opt_string: syn::Type = syn::parse_str("Option<String>").unwrap();
        let (name, is_opt) = extract_type_name(&ty_opt_string);
        assert_eq!(name, "String");
        assert!(is_opt);

        let ty_opt_i64: syn::Type = syn::parse_str("Option<i64>").unwrap();
        let (name, is_opt) = extract_type_name(&ty_opt_i64);
        assert_eq!(name, "i64");
        assert!(is_opt);

        let ty_unknown: syn::Type = syn::parse_str("&[u8]").unwrap();
        let (name, is_opt) = extract_type_name(&ty_unknown);
        assert_eq!(name, "Unknown");
        assert!(!is_opt);
    }

    #[test]
    fn test_extract_tables_from_ast_call() {
        let _ = extract_tables_from_ast();
    }
}

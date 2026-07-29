use syn::{Data, DeriveInput, Fields, spanned::Spanned};

/// Split a token string at top-level commas, ignoring commas that
/// appear inside matched parentheses. This lets us keep arguments of
/// calls like `soft_delete(field = "a", value = "0")` together while
/// still separating the outer attributes.
fn split_top_level(input: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut depth: i32 = 0;
    for c in input.chars() {
        match c {
            '(' => {
                depth += 1;
                buf.push(c);
            }
            ')' => {
                if depth > 0 {
                    depth -= 1;
                }
                buf.push(c);
            }
            ',' if depth == 0 => {
                parts.push(std::mem::take(&mut buf));
            }
            other => buf.push(other),
        }
    }
    if !buf.is_empty() {
        parts.push(buf);
    }
    parts
}

/// If `input` looks like `<name>(<inner>)`, return the inner portion
/// (with surrounding whitespace trimmed). Returns `None` otherwise.
fn strip_outer_call(input: &str, name: &str) -> Option<String> {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix(name) {
        let rest = rest.trim_start();
        if rest.starts_with('(') && rest.ends_with(')') {
            let inner = &rest[1..rest.len() - 1];
            return Some(inner.trim().to_string());
        }
    }
    None
}

/// Validates that a relation attribute has valid syntax
fn validate_relation_attribute(
    key: &str,
    value: &str,
    span: proc_macro2::Span,
) -> Result<(), syn::Error> {
    match key {
        "has_many" | "has_one" | "belongs_to" | "belongs_to_many" | "morph_many" | "morph_one" => {
            if value.is_empty() {
                return Err(syn::Error::new(
                    span,
                    format!("{} requires a model name", key),
                ));
            }
            // Check if value looks like a valid Rust identifier
            if !value
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
            {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "{} model name should start with uppercase (PascalCase)",
                        key
                    ),
                ));
            }
        }
        "foreign_key" | "related_key" | "pivot_table" | "local_key" | "name"
            if value.is_empty() =>
        {
            return Err(syn::Error::new(span, format!("{} requires a value", key)));
        }
        _ => {}
    }
    Ok(())
}

pub struct ParsedModel {
    pub name: syn::Ident,
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
    /// Soft delete configuration. `column` defaults to `deleted_at`,
    /// `value` is the literal SQL expression for the "not deleted" sentinel
    /// (e.g. `0`, `false`, `null`, `'N'`), `delval` is the literal SQL
    /// expression for the "deleted" sentinel (e.g. `1`, `true`, `now()`,
    /// `UNIX_TIMESTAMP()`). All values are emitted as raw SQL fragments
    /// (never user input) so the user is responsible for keeping them
    /// safe and portable across MySQL / PostgreSQL / SQLite.
    pub soft_delete: Option<SoftDeleteConfig>,

    pub normal_fields: Vec<syn::Ident>,
    pub normal_fields_types: Vec<syn::Type>,
    pub hidden_fields: Vec<syn::Ident>,
    /// Fields tagged with `#[orm(skip)]` or `#[sqlx(skip)]`. They are
    /// still part of the struct but excluded from generated INSERT /
    /// UPDATE statements, the `*Column` enum and JSON serialisation.
    /// Tracked for introspection; not currently consumed by the
    /// codegen because the parser already moves them out of
    /// `normal_fields` before the generators run.
    #[allow(dead_code)]
    pub skipped_fields: Vec<syn::Ident>,
    pub relations: Vec<ParsedRelation>,
    pub has_soft_deletes: bool,
    pub rag_context_fields: Vec<syn::Ident>,
    pub embedding_for: Option<(syn::Ident, String)>,
}

#[derive(Clone, Debug)]
pub struct SoftDeleteConfig {
    pub column: String,
    /// SQL fragment representing the "not deleted" state.
    /// When this is the literal string `null` (case-insensitive) the
    /// generated `SELECT` / `restore` statements compare the column
    /// against `IS NULL`. For all other values the comparison is
    /// `<column> = <value>`.
    pub value: String,
    /// SQL fragment representing the "deleted" state. This is
    /// interpolated verbatim into the generated `UPDATE` statement,
    /// so users can use database functions such as `now()`,
    /// `CURRENT_TIMESTAMP`, `UNIX_TIMESTAMP()` etc.
    pub delval: String,
}

pub struct ParsedRelation {
    pub field_name: syn::Ident,
    pub rel_type: String,
    pub rel_model: String,
    pub foreign_key: String,
    pub local_key: String,
    pub related_key: String,
    pub pivot_table: String,
    pub morph_name: String,
    pub cascade_soft_delete: bool,
}

pub fn parse(input: &DeriveInput) -> Result<ParsedModel, syn::Error> {
    let name = input.ident.clone();
    let mut table_name = format!("{}s", name.to_string().to_lowercase());
    let mut global_scope = String::new();
    let mut tenant_column = String::new();
    let mut auditable = false;
    let mut searchable = false;
    let mut policy = String::new();
    let mut before_save = String::new();
    let mut after_save = String::new();
    let mut before_delete = String::new();
    let mut after_delete = String::new();
    let mut after_fetch = String::new();
    let mut soft_delete: Option<SoftDeleteConfig> = None;

    for attr in &input.attrs {
        if attr.path().is_ident("orm") {
            let token_str = match attr.meta.require_list() {
                Ok(list) => list.tokens.to_string(),
                Err(_) => continue, // Skip malformed attributes
            };
            // Split top-level orm attributes, but keep parenthesised
            // groups such as `soft_delete(field = "...", value = "0", delval = "1")`
            // intact so the inner key/value pairs aren't accidentally cut
            // by the comma split.
            let top_parts = split_top_level(&token_str);
            for part in top_parts {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed == "auditable" {
                    auditable = true;
                } else if trimmed == "searchable" {
                    searchable = true;
                } else if let Some(inner) = strip_outer_call(trimmed, "soft_delete") {
                    let mut column: Option<String> = None;
                    let mut value: Option<String> = None;
                    let mut delval: Option<String> = None;
                    for kv in split_top_level(&inner) {
                        let kv = kv.trim();
                        if kv.is_empty() {
                            continue;
                        }
                        let kv_parts: Vec<&str> = kv.splitn(2, '=').collect();
                        if kv_parts.len() != 2 {
                            continue;
                        }
                        let k = kv_parts[0].trim();
                        let v = kv_parts[1].trim().trim_matches('"');
                        match k {
                            "field" | "column" => column = Some(v.to_string()),
                            "value" => value = Some(v.to_string()),
                            "delval" => delval = Some(v.to_string()),
                            _ => {}
                        }
                    }
                    soft_delete = Some(SoftDeleteConfig {
                        column: column.unwrap_or_else(|| "deleted_at".to_string()),
                        value: value.unwrap_or_default(),
                        delval: delval.unwrap_or_default(),
                    });
                } else {
                    let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        let key = parts[0].trim();
                        let val = parts[1].trim().trim_matches('"');
                        match key {
                            "table" | "table_name" => {
                                if val.trim().is_empty() {
                                    return Err(syn::Error::new_spanned(
                                        attr,
                                        "table name cannot be empty",
                                    ));
                                }
                                table_name = val.to_string();
                            }
                            "global_scope" => global_scope = val.to_string(),
                            "tenant_column" => tenant_column = val.to_string(),
                            "policy" => policy = val.to_string(),
                            "before_save" => before_save = val.to_string(),
                            "after_save" => after_save = val.to_string(),
                            "before_delete" => before_delete = val.to_string(),
                            "after_delete" => after_delete = val.to_string(),
                            "after_fetch" => after_fetch = val.to_string(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "Orm macro only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "Orm macro can only be used on structs",
            ));
        }
    };

    let mut normal_fields = vec![];
    let mut normal_fields_types = vec![];
    let mut hidden_fields = vec![];
    let mut skipped_fields = vec![];
    let mut relations = vec![];
    let mut rag_context_fields = vec![];
    let mut embedding_for = None;
    // If the user explicitly opted in via `#[orm(soft_delete(...))]` the
    // `has_soft_deletes` flag is derived from that. Otherwise we keep the
    // legacy behaviour of detecting a `deleted_at` field by name so
    // existing models keep working without changes.
    let mut has_soft_deletes = soft_delete.is_some();
    // Track the column name that should be considered the soft delete
    // marker. Used at the end of field iteration to synthesise a
    // default `SoftDeleteConfig` for legacy `deleted_at` models so the
    // downstream generators can always assume `soft_delete` is `Some`
    // when `has_soft_deletes` is true.
    let mut detected_soft_delete_column: Option<String> = None;

    for field in fields {
        let field_name = match field.ident.as_ref() {
            Some(ident) => ident.clone(),
            None => continue, // Skip fields without identifiers
        };
        let field_name_str = field_name.to_string();
        if field_name_str == "deleted_at" {
            has_soft_deletes = true;
            detected_soft_delete_column = Some(field_name_str.clone());
        }

        let mut is_relation = false;
        let mut rel_type = String::new();
        let mut rel_model = String::new();
        let mut foreign_key = String::new();
        let mut related_key = String::new();
        let mut pivot_table = String::new();
        let mut local_key = "id".to_string();
        let mut morph_name = String::new();
        let mut is_hidden = false;
        let mut is_skipped = false;
        let mut is_masked = false;
        let mut cascade_soft_delete = false;

        for attr in &field.attrs {
            // The macro accepts `#[orm(...)]` as well as the more
            // explicit `#[sqlx(...)]` style requested in the feature
            // spec. Both produce the same set of recognised keys.
            if attr.path().is_ident("orm") || attr.path().is_ident("sqlx") {
                let token_str = match attr.meta.require_list() {
                    Ok(list) => list.tokens.to_string(),
                    Err(_) => continue, // Skip malformed attributes
                };
                for part in split_top_level(&token_str) {
                    let trimmed = part.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if trimmed == "hidden" {
                        is_hidden = true;
                    } else if trimmed == "skip" {
                        // Excluded from generated INSERT / UPDATE column
                        // lists, bindings, JSON serialisation and the
                        // `*Column` enum. The field is still part of the
                        // struct so user code can still read/write it.
                        is_skipped = true;
                    } else if trimmed == "masked" {
                        is_masked = true;
                    } else if trimmed == "cascade_soft_delete" {
                        cascade_soft_delete = true;
                    } else if trimmed == "rag_context" {
                        rag_context_fields.push(field_name.clone());
                    } else {
                        let parts: Vec<&str> = trimmed.split('=').collect();
                        if parts.len() == 2 {
                            let key = parts[0].trim();
                            let val = parts[1].trim().trim_matches('"');
                            // Validate relation attributes
                            validate_relation_attribute(key, val, field.span())?;
                            match key {
                                "has_many" => {
                                    is_relation = true;
                                    rel_type = "has_many".to_string();
                                    rel_model = val.to_string();
                                }
                                "has_one" => {
                                    is_relation = true;
                                    rel_type = "has_one".to_string();
                                    rel_model = val.to_string();
                                }
                                "belongs_to" => {
                                    is_relation = true;
                                    rel_type = "belongs_to".to_string();
                                    rel_model = val.to_string();
                                }
                                "belongs_to_many" => {
                                    is_relation = true;
                                    rel_type = "belongs_to_many".to_string();
                                    rel_model = val.to_string();
                                }
                                "morph_many" => {
                                    is_relation = true;
                                    rel_type = "morph_many".to_string();
                                    rel_model = val.to_string();
                                }
                                "morph_one" => {
                                    is_relation = true;
                                    rel_type = "morph_one".to_string();
                                    rel_model = val.to_string();
                                }
                                "foreign_key" => foreign_key = val.to_string(),
                                "related_key" => related_key = val.to_string(),
                                "pivot_table" => pivot_table = val.to_string(),
                                "local_key" => local_key = val.to_string(),
                                "name" => morph_name = val.to_string(),
                                "embedding_for" => {
                                    embedding_for = Some((field_name.clone(), val.to_string()));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        if auditable && !is_masked && !is_skipped && !is_relation {
            let lower_name = field_name_str.to_lowercase();
            if lower_name.contains("password")
                || lower_name.contains("token")
                || lower_name.contains("secret")
                || lower_name.contains("api_key")
            {
                return Err(syn::Error::new(
                    field.span(),
                    format!(
                        "Sensitive field `{}` must be explicitly marked with `#[orm(masked)]` when `#[orm(auditable)]` is enabled.",
                        field_name_str
                    ),
                ));
            }
        }

        if is_relation {
            relations.push(ParsedRelation {
                field_name,
                rel_type,
                rel_model,
                foreign_key,
                local_key,
                related_key,
                pivot_table,
                morph_name,
                cascade_soft_delete,
            });
        } else if is_skipped {
            // Skipped fields are not exposed to the generated SQL or the
            // column enum; record the ident so downstream code (if it ever
            // needs to introspect) can still see them.
            skipped_fields.push(field_name.clone());
            if is_hidden {
                hidden_fields.push(field_name);
            }
        } else {
            normal_fields.push(field_name.clone());
            normal_fields_types.push(field.ty.clone());
            if is_hidden {
                hidden_fields.push(field_name);
            }
        }
    }

    // Synthesise a default `SoftDeleteConfig` for legacy models that
    // declared a `deleted_at` field without an explicit
    // `#[orm(soft_delete(...))]`. The defaults match the historical
    // behaviour (column = `deleted_at`, not-deleted = NULL, deleted =
    // `CURRENT_TIMESTAMP`) so all pre-existing models continue to
    // compile and behave identically.
    let soft_delete = soft_delete.or_else(|| {
        detected_soft_delete_column.map(|column| SoftDeleteConfig {
            column,
            value: String::new(),
            delval: String::new(),
        })
    });

    Ok(ParsedModel {
        name,
        table_name,
        global_scope,
        tenant_column,
        auditable,
        searchable,
        policy,
        before_save,
        after_save,
        before_delete,
        after_delete,
        after_fetch,
        soft_delete,
        normal_fields,
        normal_fields_types,
        hidden_fields,
        skipped_fields,
        relations,
        has_soft_deletes,
        rag_context_fields,
        embedding_for,
    })
}

include!("parser_tests.rs");

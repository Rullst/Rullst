use super::{ParsedRelation, SoftDeleteConfig};
use syn::{Attribute, DeriveInput, Field, spanned::Spanned};

/// Splits at top-level commas while preserving parenthesized arguments.
pub(super) fn split_top_level(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut buffer = String::new();
    let mut depth = 0_i32;

    for character in input.chars() {
        match character {
            '(' => {
                depth += 1;
                buffer.push(character);
            }
            ')' => {
                if depth > 0 {
                    depth -= 1;
                }
                buffer.push(character);
            }
            ',' if depth == 0 => parts.push(std::mem::take(&mut buffer)),
            other => buffer.push(other),
        }
    }

    if !buffer.is_empty() {
        parts.push(buffer);
    }
    parts
}

/// Returns the inner portion when `input` has the form `<name>(<inner>)`.
pub(super) fn strip_outer_call(input: &str, name: &str) -> Option<String> {
    let rest = input.trim().strip_prefix(name)?.trim_start();
    if !rest.starts_with('(') || !rest.ends_with(')') {
        return None;
    }
    Some(rest[1..rest.len() - 1].trim().to_string())
}

pub(super) fn validate_relation_attribute(
    key: &str,
    value: &str,
    span: proc_macro2::Span,
) -> Result<(), syn::Error> {
    match key {
        "has_many" | "has_one" | "belongs_to" | "belongs_to_many" | "morph_many" | "morph_one" => {
            if value.is_empty() {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "Relation attribute '{}' requires a target model name (e.g. #[orm({} = \"User\")])",
                        key, key
                    ),
                ));
            }
            if !value.chars().next().is_some_and(char::is_uppercase) {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "Relation attribute '{}' model name must start with uppercase (PascalCase, e.g. #[orm({} = \"User\")])",
                        key, key
                    ),
                ));
            }
        }
        "foreign_key" | "related_key" | "pivot_table" | "local_key" | "name"
            if value.is_empty() =>
        {
            return Err(syn::Error::new(
                span,
                format!(
                    "Attribute '{}' requires a non-empty string value (e.g. #[orm({} = \"user_id\")])",
                    key, key
                ),
            ));
        }
        _ => {}
    }
    Ok(())
}

pub(super) struct ModelAttributes {
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
        let mut parsed = Self::new(format!("{}s", input.ident.to_string().to_lowercase()));
        for attribute in input
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("orm"))
        {
            let Ok(list) = attribute.meta.require_list() else {
                continue;
            };
            for part in split_top_level(&list.tokens.to_string()) {
                parsed.apply(part.trim(), attribute)?;
            }
        }
        Ok(parsed)
    }

    fn new(table_name: String) -> Self {
        Self {
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

    fn apply(&mut self, part: &str, attribute: &Attribute) -> Result<(), syn::Error> {
        match part {
            "" => return Ok(()),
            "auditable" => {
                self.auditable = true;
                return Ok(());
            }
            "searchable" => {
                self.searchable = true;
                return Ok(());
            }
            _ => {}
        }

        if let Some(inner) = strip_outer_call(part, "soft_delete") {
            self.soft_delete = Some(parse_soft_delete(&inner));
            return Ok(());
        }

        let Some((key, value)) = key_value(part) else {
            return Ok(());
        };
        match key {
            "table" | "table_name" => {
                if value.trim().is_empty() {
                    return Err(syn::Error::new_spanned(
                        attribute,
                        "table name cannot be empty (e.g. #[orm(table = \"users\")])",
                    ));
                }
                self.table_name = value.to_string();
            }
            "tabel" | "tbl" | "tablename" => {
                return Err(syn::Error::new_spanned(
                    attribute,
                    format!(
                        "Unknown model attribute `#[orm({key} = ...)]`. Did you mean `#[orm(table = \"...\")]`?"
                    ),
                ));
            }
            "global_scope" => self.global_scope = value.to_string(),
            "tenant_column" => self.tenant_column = value.to_string(),
            "policy" => self.policy = value.to_string(),
            "before_save" => self.before_save = value.to_string(),
            "after_save" => self.after_save = value.to_string(),
            "before_delete" => self.before_delete = value.to_string(),
            "after_delete" => self.after_delete = value.to_string(),
            "after_fetch" => self.after_fetch = value.to_string(),
            _ => {}
        }
        Ok(())
    }
}

fn parse_soft_delete(input: &str) -> SoftDeleteConfig {
    let mut column = None;
    let mut value = None;
    let mut delval = None;
    for part in split_top_level(input) {
        let Some((key, parsed_value)) = key_value(part.trim()) else {
            continue;
        };
        match key {
            "field" | "column" => column = Some(parsed_value.to_string()),
            "value" => value = Some(parsed_value.to_string()),
            "delval" => delval = Some(parsed_value.to_string()),
            _ => {}
        }
    }
    SoftDeleteConfig {
        column: column.unwrap_or_else(|| "deleted_at".to_string()),
        value: value.unwrap_or_default(),
        delval: delval.unwrap_or_default(),
    }
}

pub(super) struct FieldAttributes {
    pub is_hidden: bool,
    pub is_skipped: bool,
    pub is_masked: bool,
    pub rag_context: bool,
    pub embedding_for: Option<String>,
    relation_type: String,
    relation_model: String,
    foreign_key: String,
    related_key: String,
    pivot_table: String,
    local_key: String,
    morph_name: String,
    cascade_soft_delete: bool,
}

impl FieldAttributes {
    pub fn parse(field: &Field) -> Result<Self, syn::Error> {
        let mut parsed = Self::default();
        for attribute in field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("orm") || attr.path().is_ident("sqlx"))
        {
            let Ok(list) = attribute.meta.require_list() else {
                continue;
            };
            for part in split_top_level(&list.tokens.to_string()) {
                parsed.apply(part.trim(), attribute, field.span())?;
            }
        }
        Ok(parsed)
    }

    pub fn is_relation(&self) -> bool {
        !self.relation_type.is_empty()
    }

    pub fn into_relation(self, field_name: syn::Ident) -> ParsedRelation {
        ParsedRelation {
            field_name,
            rel_type: self.relation_type,
            rel_model: self.relation_model,
            foreign_key: self.foreign_key,
            local_key: self.local_key,
            related_key: self.related_key,
            pivot_table: self.pivot_table,
            morph_name: self.morph_name,
            cascade_soft_delete: self.cascade_soft_delete,
        }
    }

    fn apply(
        &mut self,
        part: &str,
        attribute: &Attribute,
        span: proc_macro2::Span,
    ) -> Result<(), syn::Error> {
        match part {
            "" => return Ok(()),
            "hidden" => self.is_hidden = true,
            "skip" => self.is_skipped = true,
            "masked" => self.is_masked = true,
            "cascade_soft_delete" => self.cascade_soft_delete = true,
            "rag_context" => self.rag_context = true,
            _ => return self.apply_key_value(part, attribute, span),
        }
        Ok(())
    }

    fn apply_key_value(
        &mut self,
        part: &str,
        attribute: &Attribute,
        span: proc_macro2::Span,
    ) -> Result<(), syn::Error> {
        let Some((key, value)) = key_value(part) else {
            return Ok(());
        };
        validate_relation_attribute(key, value, span)?;
        match key {
            "has_many" | "has_one" | "belongs_to" | "belongs_to_many" | "morph_many"
            | "morph_one" => {
                self.relation_type = key.to_string();
                self.relation_model = value.to_string();
            }
            "hasmany" => return typo(attribute, "hasmany", "has_many"),
            "belongsto" => return typo(attribute, "belongsto", "belongs_to"),
            "hasone" => return typo(attribute, "hasone", "has_one"),
            "foreignkey" | "fk" => return typo(attribute, key, "foreign_key"),
            "foreign_key" => self.foreign_key = value.to_string(),
            "related_key" => self.related_key = value.to_string(),
            "pivot_table" => self.pivot_table = value.to_string(),
            "local_key" => self.local_key = value.to_string(),
            "name" => self.morph_name = value.to_string(),
            "embedding_for" => self.embedding_for = Some(value.to_string()),
            _ => {}
        }
        Ok(())
    }
}

impl Default for FieldAttributes {
    fn default() -> Self {
        Self {
            is_hidden: false,
            is_skipped: false,
            is_masked: false,
            rag_context: false,
            embedding_for: None,
            relation_type: String::new(),
            relation_model: String::new(),
            foreign_key: String::new(),
            related_key: String::new(),
            pivot_table: String::new(),
            local_key: "id".to_string(),
            morph_name: String::new(),
            cascade_soft_delete: false,
        }
    }
}

fn key_value(input: &str) -> Option<(&str, &str)> {
    let (key, value) = input.split_once('=')?;
    Some((key.trim(), value.trim().trim_matches('"')))
}

fn typo(attribute: &Attribute, actual: &str, expected: &str) -> Result<(), syn::Error> {
    Err(syn::Error::new_spanned(
        attribute,
        format!(
            "Unknown attribute `#[orm({actual} = ...)]`. Did you mean `#[orm({expected} = \"...\")]`?"
        ),
    ))
}

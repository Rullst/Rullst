use super::common::*;
use crate::parser::ParsedRelation;
use std::collections::HashSet;
use syn::meta::ParseNestedMeta;
use syn::{Field, spanned::Spanned};

pub(in crate::parser) struct FieldAttributes {
    pub is_hidden: bool,
    pub is_skipped: bool,
    pub is_masked: bool,
    pub is_encrypted: bool,
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
        let mut seen = HashSet::new();
        for attribute in field.attrs.iter().filter(|attribute| {
            attribute.path().is_ident("orm") || attribute.path().is_ident("sqlx")
        }) {
            if attribute.path().is_ident("orm") {
                attribute.parse_nested_meta(|meta| parsed.apply_orm(meta, &mut seen))?;
            } else {
                attribute.parse_nested_meta(|meta| parsed.apply_sqlx(meta, &mut seen))?;
            }
        }
        parsed.validate(&seen, field.span())?;
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

    fn apply_orm(
        &mut self,
        meta: ParseNestedMeta<'_>,
        seen: &mut HashSet<&'static str>,
    ) -> Result<(), syn::Error> {
        let key = path_name(&meta)?;
        match key.as_str() {
            "hidden" => {
                mark_once(seen, "hidden", &meta)?;
                self.is_hidden = true;
            }
            "skip" => {
                mark_once(seen, "skip", &meta)?;
                self.is_skipped = true;
            }
            "masked" => {
                mark_once(seen, "masked", &meta)?;
                self.is_masked = true;
            }
            "encrypted" => {
                mark_once(seen, "encrypted", &meta)?;
                self.is_encrypted = true;
            }
            "cascade_soft_delete" => {
                mark_once(seen, "cascade_soft_delete", &meta)?;
                self.cascade_soft_delete = true;
            }
            "rag_context" => {
                mark_once(seen, "rag_context", &meta)?;
                self.rag_context = true;
            }
            // Encoding remains owned by the field type and SQLx. This marker
            // exists for documented compatibility and has no hidden runtime.
            "json" => mark_once(seen, "json", &meta)?,
            _ => return self.apply_orm_value(key, meta, seen),
        }
        Ok(())
    }

    fn apply_orm_value(
        &mut self,
        key: String,
        meta: ParseNestedMeta<'_>,
        seen: &mut HashSet<&'static str>,
    ) -> Result<(), syn::Error> {
        match key.as_str() {
            "has_many" | "has_one" | "belongs_to" | "belongs_to_many" | "morph_many"
            | "morph_one" | "morph_to" => {
                mark_once(seen, "relation", &meta)?;
                let value = string_value(&meta)?;
                validate_relation_attribute(&key, &value, meta.path.span())?;
                self.relation_type = key;
                self.relation_model = value;
            }
            "hasmany" => return Err(typo(&meta, "hasmany", "has_many")),
            "belongsto" => return Err(typo(&meta, "belongsto", "belongs_to")),
            "hasone" => return Err(typo(&meta, "hasone", "has_one")),
            "foreignkey" | "fk" => return Err(typo(&meta, &key, "foreign_key")),
            "foreign_key" => {
                self.foreign_key = relation_identifier(&meta, seen, "foreign_key")?;
            }
            "related_key" => {
                self.related_key = relation_identifier(&meta, seen, "related_key")?;
            }
            "pivot_table" => {
                self.pivot_table = relation_identifier(&meta, seen, "pivot_table")?;
            }
            "local_key" => {
                self.local_key = relation_identifier(&meta, seen, "local_key")?;
            }
            "name" | "morph_name" => {
                self.morph_name = relation_identifier(&meta, seen, "morph_name")?;
            }
            "embedding_for" => {
                self.embedding_for = Some(identifier_value(&meta, seen, "embedding_for")?);
            }
            _ => return Err(meta.error(format!("unsupported ORM field option `{key}`"))),
        }
        Ok(())
    }

    fn apply_sqlx(
        &mut self,
        meta: ParseNestedMeta<'_>,
        seen: &mut HashSet<&'static str>,
    ) -> Result<(), syn::Error> {
        let key = path_name(&meta)?;
        match key.as_str() {
            "skip" => {
                mark_once(seen, "skip", &meta)?;
                self.is_skipped = true;
            }
            "default" => mark_once(seen, "sqlx_default", &meta)?,
            "json" => {
                mark_once(seen, "json", &meta)?;
                if meta.input.peek(syn::token::Paren) {
                    let mut nullable = false;
                    meta.parse_nested_meta(|nested| {
                        if !nested.path.is_ident("nullable") || nullable {
                            return Err(nested
                                .error("SQLx json accepts only one optional `nullable` modifier"));
                        }
                        nullable = true;
                        Ok(())
                    })?;
                }
            }
            "rename" | "try_from" | "flatten" => {
                return Err(meta.error(format!(
                    "#[sqlx({key})] is not supported by #[derive(Orm)]; use matching persisted field names and types"
                )));
            }
            _ => return Err(meta.error(format!("unsupported SQLx field option `{key}`"))),
        }
        Ok(())
    }

    fn validate(
        &self,
        seen: &HashSet<&'static str>,
        span: proc_macro2::Span,
    ) -> Result<(), syn::Error> {
        let relation_option = [
            "foreign_key",
            "related_key",
            "pivot_table",
            "local_key",
            "morph_name",
            "cascade_soft_delete",
        ]
        .into_iter()
        .any(|option| seen.contains(option));
        if !self.is_relation() && relation_option {
            return Err(syn::Error::new(
                span,
                "relationship options require exactly one relation declaration",
            ));
        }
        if self.cascade_soft_delete
            && !matches!(self.relation_type.as_str(), "has_many" | "has_one")
        {
            return Err(syn::Error::new(
                span,
                "cascade_soft_delete is supported only on has_many or has_one relations",
            ));
        }
        if !self.pivot_table.is_empty() && self.relation_type != "belongs_to_many" {
            return Err(syn::Error::new(
                span,
                "pivot_table is supported only on belongs_to_many relations",
            ));
        }
        if self.relation_type == "belongs_to_many" && self.pivot_table.is_empty() {
            return Err(syn::Error::new(
                span,
                "belongs_to_many requires pivot_table = \"...\"",
            ));
        }
        if !self.morph_name.is_empty()
            && !matches!(
                self.relation_type.as_str(),
                "morph_many" | "morph_one" | "morph_to"
            )
        {
            return Err(syn::Error::new(
                span,
                "morph_name is supported only on polymorphic relations",
            ));
        }
        if self.embedding_for.is_some() && (self.is_relation() || self.is_skipped) {
            return Err(syn::Error::new(
                span,
                "embedding_for requires a persisted non-relation field",
            ));
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
            is_encrypted: false,
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

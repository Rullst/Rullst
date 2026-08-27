mod attributes;

use attributes::{FieldAttributes, ModelAttributes};
#[cfg(test)]
use attributes::{split_top_level, strip_outer_call, validate_relation_attribute};
use syn::{Data, DeriveInput, Fields, spanned::Spanned};

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
    let mut model_attributes = ModelAttributes::parse(input)?;

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
    let mut has_soft_deletes = model_attributes.soft_delete.is_some();
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

        let field_attributes = FieldAttributes::parse(field)?;
        if field_attributes.rag_context {
            rag_context_fields.push(field_name.clone());
        }
        if let Some(target) = field_attributes.embedding_for.clone() {
            embedding_for = Some((field_name.clone(), target));
        }

        if model_attributes.auditable
            && !field_attributes.is_masked
            && !field_attributes.is_skipped
            && !field_attributes.is_relation()
        {
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

        if field_attributes.is_relation() {
            relations.push(field_attributes.into_relation(field_name));
        } else if field_attributes.is_skipped {
            // Skipped fields are not exposed to the generated SQL or the
            // column enum; record the ident so downstream code (if it ever
            // needs to introspect) can still see them.
            skipped_fields.push(field_name.clone());
            if field_attributes.is_hidden {
                hidden_fields.push(field_name);
            }
        } else {
            normal_fields.push(field_name.clone());
            normal_fields_types.push(field.ty.clone());
            if field_attributes.is_hidden {
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
    let soft_delete = model_attributes.soft_delete.take().or_else(|| {
        detected_soft_delete_column.map(|column| SoftDeleteConfig {
            column,
            value: String::new(),
            delval: String::new(),
        })
    });

    Ok(ParsedModel {
        name,
        table_name: model_attributes.table_name,
        global_scope: model_attributes.global_scope,
        tenant_column: model_attributes.tenant_column,
        auditable: model_attributes.auditable,
        searchable: model_attributes.searchable,
        policy: model_attributes.policy,
        before_save: model_attributes.before_save,
        after_save: model_attributes.after_save,
        before_delete: model_attributes.before_delete,
        after_delete: model_attributes.after_delete,
        after_fetch: model_attributes.after_fetch,
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

mod attributes;

use attributes::{FieldAttributes, ModelAttributes};
#[cfg(test)]
use attributes::{split_top_level, strip_outer_call, validate_relation_attribute};
use syn::{Data, DeriveInput, Fields, spanned::Spanned};

pub struct ParsedModel {
    pub name: syn::Ident,
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
    /// String fields transparently encrypted by generated persistence methods.
    pub encrypted_fields: Vec<ParsedEncryptedField>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncryptedFieldKind {
    String,
    OptionalString,
}

pub struct ParsedEncryptedField {
    pub name: syn::Ident,
    pub kind: EncryptedFieldKind,
}

fn encrypted_field_kind(field_type: &syn::Type) -> Option<EncryptedFieldKind> {
    let syn::Type::Path(type_path) = field_type else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident == "String" && matches!(segment.arguments, syn::PathArguments::None) {
        return Some(EncryptedFieldKind::String);
    }
    if segment.ident != "Option" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    if arguments.args.len() != 1 {
        return None;
    }
    let syn::GenericArgument::Type(syn::Type::Path(inner_path)) = &arguments.args[0] else {
        return None;
    };
    let inner = inner_path.path.segments.last()?;
    (inner.ident == "String" && matches!(inner.arguments, syn::PathArguments::None))
        .then_some(EncryptedFieldKind::OptionalString)
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
    let mut encrypted_fields = vec![];
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
        if field_attributes.is_encrypted {
            if field_attributes.is_relation() || field_attributes.is_skipped {
                return Err(syn::Error::new(
                    field.span(),
                    "#[orm(encrypted)] cannot be combined with relation or skipped fields",
                ));
            }
            let kind = encrypted_field_kind(&field.ty).ok_or_else(|| {
                syn::Error::new(
                    field.ty.span(),
                    "#[orm(encrypted)] supports only String and Option<String> fields",
                )
            })?;
            encrypted_fields.push(ParsedEncryptedField {
                name: field_name.clone(),
                kind,
            });
        }
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

    if !model_attributes.tenant_column.is_empty() {
        let Some((_, tenant_type)) = normal_fields
            .iter()
            .zip(normal_fields_types.iter())
            .find(|(field, _)| *field == model_attributes.tenant_column.as_str())
        else {
            return Err(syn::Error::new_spanned(
                input,
                format!(
                    "tenant_column `{}` must name a persisted field on the model",
                    model_attributes.tenant_column
                ),
            ));
        };
        let supported = match tenant_type {
            syn::Type::Path(path) if path.qself.is_none() => {
                path.path.segments.last().is_some_and(|segment| {
                    matches!(
                        segment.ident.to_string().as_str(),
                        "String" | "i32" | "f64" | "bool"
                    )
                })
            }
            _ => false,
        };
        if !supported {
            return Err(syn::Error::new_spanned(
                tenant_type,
                "tenant_column supports String, i32, f64, or bool so it can be bound without lossy conversion",
            ));
        }
    }

    for relation in &relations {
        if matches!(
            relation.rel_type.as_str(),
            "morph_many" | "morph_one" | "morph_to"
        ) && relation.morph_name.is_empty()
        {
            return Err(syn::Error::new_spanned(
                input,
                format!(
                    "polymorphic relation `{}` requires `morph_name = \"...\"` (the legacy alias `name` is also accepted)",
                    relation.field_name
                ),
            ));
        }

        if relation.rel_type != "morph_to" {
            continue;
        }

        let morph_id_column = if relation.foreign_key.is_empty() {
            format!("{}_id", relation.morph_name)
        } else {
            relation.foreign_key.clone()
        };
        let morph_type_column = format!("{}_type", relation.morph_name);
        let persisted_type = |column: &str| {
            normal_fields
                .iter()
                .zip(normal_fields_types.iter())
                .find(|(field, _)| *field == column)
                .map(|(_, field_type)| field_type)
        };

        let Some(morph_id_type) = persisted_type(&morph_id_column) else {
            return Err(syn::Error::new_spanned(
                input,
                format!(
                    "morph_to relation `{}` requires persisted id field `{}`",
                    relation.field_name, morph_id_column
                ),
            ));
        };
        let id_is_bindable = match morph_id_type {
            syn::Type::Path(path) if path.qself.is_none() => {
                path.path.segments.last().is_some_and(|segment| {
                    matches!(
                        segment.ident.to_string().as_str(),
                        "String" | "i32" | "f64" | "bool"
                    )
                })
            }
            _ => false,
        };
        if !id_is_bindable {
            return Err(syn::Error::new_spanned(
                morph_id_type,
                "morph_to id fields support String, i32, f64, or bool so they can be bound without lossy conversion",
            ));
        }

        let Some(morph_type_type) = persisted_type(&morph_type_column) else {
            return Err(syn::Error::new_spanned(
                input,
                format!(
                    "morph_to relation `{}` requires persisted discriminator field `{}`",
                    relation.field_name, morph_type_column
                ),
            ));
        };
        let discriminator_is_string = match morph_type_type {
            syn::Type::Path(path) if path.qself.is_none() => path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "String"),
            _ => false,
        };
        if !discriminator_is_string {
            return Err(syn::Error::new_spanned(
                morph_type_type,
                "morph_to discriminator fields must use String",
            ));
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
        backend: model_attributes.backend,
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
        encrypted_fields,
        skipped_fields,
        relations,
        has_soft_deletes,
        rag_context_fields,
        embedding_for,
    })
}

include!("parser_tests.rs");

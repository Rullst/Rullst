//! Fail-closed validation for application-registered Nexus model metadata.

use std::collections::BTreeSet;

use super::{FieldKind, NexusBuildError, RegistryEntry};

const MAX_REGISTERED_MODELS: usize = 128;
const MAX_FIELDS_PER_MODEL: usize = 128;
const MAX_LABEL_BYTES: usize = 256;
const MAX_ICON_BYTES: usize = 64;
const MAX_ENUM_OPTIONS: usize = 128;
const MAX_ENUM_OPTION_BYTES: usize = 128;
const MAX_KEY_COLUMNS: usize = 8;

pub(super) fn validate_registry(registry: &[RegistryEntry]) -> Result<(), NexusBuildError> {
    if registry.len() > MAX_REGISTERED_MODELS {
        return invalid("the registry exceeds 128 models");
    }

    let mut tables = BTreeSet::new();
    for entry in registry {
        validate_identifier(
            entry.table,
            "a table name is not a bounded ASCII identifier",
        )?;
        validate_identifier(
            entry.pk,
            "a primary-key name is not a bounded ASCII identifier",
        )?;
        validate_key_columns(entry)?;
        if let Some(tenant_column) = entry.tenant_column {
            validate_identifier(
                tenant_column,
                "a tenant column is not a bounded ASCII identifier",
            )?;
        }
        validate_display_text(
            entry.label,
            MAX_LABEL_BYTES,
            "a model label is empty, oversized, or contains control characters",
        )?;
        validate_display_text(
            entry.icon,
            MAX_ICON_BYTES,
            "a model icon is empty, oversized, or contains control characters",
        )?;
        if !tables.insert(entry.table) {
            return invalid("the same table was registered more than once");
        }
        validate_fields(entry)?;
    }
    Ok(())
}

fn validate_fields(entry: &RegistryEntry) -> Result<(), NexusBuildError> {
    if entry.fields.is_empty() || entry.fields.len() > MAX_FIELDS_PER_MODEL {
        return invalid("a registered model must expose between 1 and 128 fields");
    }
    let mut names = BTreeSet::new();
    for field in &entry.fields {
        validate_identifier(field.name, "a field name is not a bounded ASCII identifier")?;
        validate_display_text(
            field.label,
            MAX_LABEL_BYTES,
            "a field label is empty, oversized, or contains control characters",
        )?;
        if !names.insert(field.name) {
            return invalid("a registered model contains duplicate field names");
        }
        validate_field_kind(&field.kind)?;
    }
    if !entry
        .key_columns()
        .iter()
        .all(|column| names.contains(column))
    {
        return invalid("the registered primary key is not present in the field metadata");
    }
    if let Some(tenant_column) = entry.tenant_column {
        let tenant_field = entry
            .fields
            .iter()
            .find(|field| field.name == tenant_column)
            .ok_or(NexusBuildError::InvalidModelMetadata {
                reason: "the registered tenant column is absent from field metadata",
            })?;
        if !tenant_field.readonly
            || entry.is_key_column(tenant_column)
            || !matches!(tenant_field.kind, FieldKind::Text)
        {
            return invalid(
                "a tenant column must be a readonly text non-primary-key field controlled by Nexus",
            );
        }
    }
    Ok(())
}

/// A composite key lists distinct identifiers, at most eight, starting with
/// `pk`, so `pk` and the key columns can never disagree.
fn validate_key_columns(entry: &RegistryEntry) -> Result<(), NexusBuildError> {
    let columns = &entry.pk_columns;
    if columns.len() > MAX_KEY_COLUMNS {
        return invalid("a primary key has more than 8 columns");
    }
    if columns.first().is_some_and(|first| *first != entry.pk) {
        return invalid("the first primary-key column differs from the registered primary key");
    }
    let mut unique = BTreeSet::new();
    for column in columns {
        validate_identifier(
            column,
            "a primary-key column is not a bounded ASCII identifier",
        )?;
        if !unique.insert(*column) {
            return invalid("a primary-key column is listed more than once");
        }
    }
    Ok(())
}

fn validate_field_kind(kind: &FieldKind) -> Result<(), NexusBuildError> {
    match kind {
        FieldKind::Enum { options } => {
            if options.is_empty() || options.len() > MAX_ENUM_OPTIONS {
                return invalid("an enum widget must expose between 1 and 128 options");
            }
            let mut unique = BTreeSet::new();
            for option in options {
                validate_display_text(
                    option,
                    MAX_ENUM_OPTION_BYTES,
                    "an enum option is empty, oversized, or contains control characters",
                )?;
                if !unique.insert(*option) {
                    return invalid("an enum widget contains duplicate options");
                }
            }
            Ok(())
        }
        FieldKind::Integer { min, max } if min > max => {
            invalid("an integer widget's minimum exceeds its maximum")
        }
        FieldKind::ForeignKey { table, label_col } => {
            validate_identifier(
                table,
                "a foreign-key table is not a bounded ASCII identifier",
            )?;
            validate_identifier(
                label_col,
                "a foreign-key label column is not a bounded ASCII identifier",
            )
        }
        _ => Ok(()),
    }
}

fn validate_identifier(value: &str, reason: &'static str) -> Result<(), NexusBuildError> {
    let mut bytes = value.bytes();
    let starts_correctly = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_');
    if !starts_correctly
        || value.len() > 64
        || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return invalid(reason);
    }
    Ok(())
}

fn validate_display_text(
    value: &str,
    maximum: usize,
    reason: &'static str,
) -> Result<(), NexusBuildError> {
    if value.is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        return invalid(reason);
    }
    Ok(())
}

fn invalid<T>(reason: &'static str) -> Result<T, NexusBuildError> {
    Err(NexusBuildError::InvalidModelMetadata { reason })
}

#[cfg(test)]
#[path = "metadata_bounds_tests.rs"]
mod bounds_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nexus::{FieldMeta, NexusBuildError};

    fn valid_entry() -> RegistryEntry {
        RegistryEntry {
            table: "articles",
            label: "Articles",
            icon: "A",
            pk: "id",
            pk_columns: Vec::new(),
            tenant_column: None,
            fields: vec![
                FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
                FieldMeta::new(
                    "status",
                    "Status",
                    FieldKind::Enum {
                        options: vec!["draft", "published"],
                    },
                ),
            ],
        }
    }

    #[test]
    fn accepts_bounded_registered_metadata() {
        validate_registry(&[valid_entry()]).expect("valid registry");
    }

    #[test]
    fn rejects_ambiguous_or_unsafe_registered_metadata() {
        let mut duplicate = valid_entry();
        duplicate.table = "articles";
        assert!(matches!(
            validate_registry(&[valid_entry(), duplicate]),
            Err(NexusBuildError::InvalidModelMetadata { .. })
        ));

        let mut invalid_identifier = valid_entry();
        invalid_identifier.fields[1].name = "status;DROP";
        assert!(validate_registry(&[invalid_identifier]).is_err());

        let mut duplicate_enum = valid_entry();
        duplicate_enum.fields[1].kind = FieldKind::Enum {
            options: vec!["draft", "draft"],
        };
        assert!(validate_registry(&[duplicate_enum]).is_err());

        let mut inverted_integer = valid_entry();
        inverted_integer.fields[0].kind = FieldKind::Integer { min: 1, max: 0 };
        assert!(validate_registry(&[inverted_integer]).is_err());
        let mut single_integer = valid_entry();
        single_integer.fields[0].kind = FieldKind::Integer { min: 7, max: 7 };
        assert!(validate_registry(&[single_integer]).is_ok());

        let mut invalid_tenant = valid_entry();
        invalid_tenant.tenant_column = Some("status");
        assert!(validate_registry(&[invalid_tenant]).is_err());

        let mut valid_tenant = valid_entry();
        valid_tenant.tenant_column = Some("status");
        valid_tenant.fields[1] = FieldMeta::new("status", "Tenant", FieldKind::Text)
            .readonly()
            .hidden();
        assert!(validate_registry(&[valid_tenant]).is_ok());
    }

    /// `progress (user_id, lesson_id)` with a tenant column and eight
    /// further integer columns `k0`..`k7`.
    fn composite_entry() -> RegistryEntry {
        let mut fields = vec![
            FieldMeta::new("user_id", "User", FieldKind::Number),
            FieldMeta::new("lesson_id", "Lesson", FieldKind::Number),
            FieldMeta::new("tenant_id", "Tenant", FieldKind::Text)
                .readonly()
                .hidden(),
        ];
        for name in ["k0", "k1", "k2", "k3", "k4", "k5", "k6", "k7"] {
            fields.push(FieldMeta::new(name, "Key", FieldKind::Number));
        }
        RegistryEntry {
            table: "progress",
            label: "Progress",
            icon: "P",
            pk: "user_id",
            pk_columns: vec!["user_id", "lesson_id"],
            tenant_column: Some("tenant_id"),
            fields,
        }
    }

    fn reason(entry: RegistryEntry) -> Option<&'static str> {
        match validate_registry(&[entry]) {
            Ok(()) => None,
            Err(NexusBuildError::InvalidModelMetadata { reason }) => Some(reason),
            Err(other) => panic!("unexpected error {other:?}"),
        }
    }

    #[test]
    fn composite_keys_are_bounded_distinct_registered_columns() {
        assert_eq!(reason(composite_entry()), None);

        let mut eight = composite_entry();
        eight.pk = "k0";
        eight.pk_columns = vec!["k0", "k1", "k2", "k3", "k4", "k5", "k6", "k7"];
        assert_eq!(reason(eight), None);
        let mut nine = composite_entry();
        nine.pk_columns = vec!["user_id", "k0", "k1", "k2", "k3", "k4", "k5", "k6", "k7"];
        assert_eq!(reason(nine), Some("a primary key has more than 8 columns"));

        let mut reordered = composite_entry();
        reordered.pk_columns = vec!["lesson_id", "user_id"];
        assert_eq!(
            reason(reordered),
            Some("the first primary-key column differs from the registered primary key")
        );

        let mut repeated = composite_entry();
        repeated.pk_columns = vec!["user_id", "user_id"];
        assert_eq!(
            reason(repeated),
            Some("a primary-key column is listed more than once")
        );

        let mut unsafe_name = composite_entry();
        unsafe_name.pk_columns = vec!["user_id", "lesson_id;DROP"];
        assert_eq!(
            reason(unsafe_name),
            Some("a primary-key column is not a bounded ASCII identifier")
        );

        let mut unregistered = composite_entry();
        unregistered.pk_columns = vec!["user_id", "course_id"];
        assert_eq!(
            reason(unregistered),
            Some("the registered primary key is not present in the field metadata")
        );

        let mut tenant_key = composite_entry();
        tenant_key.pk_columns = vec!["user_id", "tenant_id"];
        assert_eq!(
            reason(tenant_key),
            Some(
                "a tenant column must be a readonly text non-primary-key field controlled by Nexus"
            )
        );

        // A single key named only by `pk` still has to be registered.
        let mut missing = valid_entry();
        missing.pk = "uuid";
        assert_eq!(
            reason(missing),
            Some("the registered primary key is not present in the field metadata")
        );
    }
}

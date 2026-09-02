//! Fail-closed validation for application-registered Nexus model metadata.

use std::collections::BTreeSet;

use super::{FieldKind, NexusBuildError, RegistryEntry};

const MAX_REGISTERED_MODELS: usize = 128;
const MAX_FIELDS_PER_MODEL: usize = 128;
const MAX_LABEL_BYTES: usize = 256;
const MAX_ICON_BYTES: usize = 64;
const MAX_ENUM_OPTIONS: usize = 128;
const MAX_ENUM_OPTION_BYTES: usize = 128;

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
    let mut has_primary_key = false;
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
        has_primary_key |= field.name == entry.pk;
        validate_field_kind(&field.kind)?;
    }
    if !has_primary_key {
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
            || tenant_column == entry.pk
            || !matches!(tenant_field.kind, FieldKind::Text)
        {
            return invalid(
                "a tenant column must be a readonly text non-primary-key field controlled by Nexus",
            );
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
mod tests {
    use super::*;
    use crate::nexus::{FieldMeta, NexusBuildError};

    fn valid_entry() -> RegistryEntry {
        RegistryEntry {
            table: "articles",
            label: "Articles",
            icon: "A",
            pk: "id",
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
}

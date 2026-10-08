//! The personal-data inventory: model fields classified with the ORM
//! attributes, plus a name heuristic for unclassified likely personal data.
//!
//! Recognized markers (from `rullst-orm-macros`): `#[derive(PersonalData)]`
//! with `#[privacy]` (or `#[privacy(...)]`) on a field, `#[orm(encrypted)]`
//! or a `SecretString` field type (stored encrypted unless `#[orm(skip)]` or
//! `#[sqlx(skip)]`), and `#[orm(masked)]` (redacted from audit records).

use proc_macro2::TokenTree;

use crate::generators::audit_compliance::EvidenceStatus;

use super::model::{PersonalData, PersonalField};
use super::sources::ProjectSources;

/// Field-name segments that usually hold personal data. A name heuristic only.
const LIKELY_PERSONAL: [&str; 9] = [
    "email", "phone", "cpf", "cnpj", "ssn", "birth", "address", "document", "ip",
];

/// Whether a snake_case field name contains a likely personal-data segment:
/// `ip` must be a whole segment (`client_ip`, not `zip`); the others may
/// start one (`emails`, `birthdate`, `phone_number`).
pub(super) fn likely_personal(name: &str) -> bool {
    name.to_ascii_lowercase().split('_').any(|segment| {
        LIKELY_PERSONAL.iter().any(|marker| {
            if *marker == "ip" {
                segment == "ip"
            } else {
                segment.starts_with(marker)
            }
        })
    })
}

#[derive(Default)]
struct Markers {
    privacy: bool,
    encrypted: bool,
    masked: bool,
    skipped: bool,
}

/// The top-level identifiers inside `#[name(...)]`.
fn list_idents(attribute: &syn::Attribute) -> Vec<String> {
    match &attribute.meta {
        syn::Meta::List(list) => list
            .tokens
            .clone()
            .into_iter()
            .filter_map(|token| match token {
                TokenTree::Ident(ident) => Some(ident.to_string()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn last_segment(path: &syn::Path) -> Option<String> {
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
}

fn is_secret_string(ty: &syn::Type) -> bool {
    let syn::Type::Path(path) = ty else {
        return false;
    };
    let Some(segment) = path.path.segments.last() else {
        return false;
    };
    if segment.ident == "SecretString" {
        return true;
    }
    // `Option<SecretString>`
    match &segment.arguments {
        syn::PathArguments::AngleBracketed(arguments) if segment.ident == "Option" => {
            arguments.args.iter().any(|argument| {
                matches!(argument, syn::GenericArgument::Type(inner) if is_secret_string(inner))
            })
        }
        _ => false,
    }
}

fn markers(field: &syn::Field) -> Markers {
    let mut markers = Markers {
        encrypted: is_secret_string(&field.ty),
        ..Markers::default()
    };
    for attribute in &field.attrs {
        let path = attribute.path();
        if path.is_ident("privacy") {
            markers.privacy = true;
        } else if path.is_ident("orm") || path.is_ident("sqlx") {
            let orm = path.is_ident("orm");
            for ident in list_idents(attribute) {
                match ident.as_str() {
                    "encrypted" if orm => markers.encrypted = true,
                    "masked" if orm => markers.masked = true,
                    "skip" => markers.skipped = true,
                    _ => {}
                }
            }
        }
    }
    markers.encrypted &= !markers.skipped;
    markers
}

fn derives(attributes: &[syn::Attribute]) -> Vec<String> {
    let mut names = Vec::new();
    for attribute in attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("derive"))
    {
        let _ = attribute.parse_nested_meta(|meta| {
            if let Some(name) = last_segment(&meta.path) {
                names.push(name);
            }
            Ok(())
        });
    }
    names
}

fn visit(items: &[syn::Item], file: &str, fields: &mut Vec<PersonalField>) {
    for item in items {
        match item {
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    visit(items, file, fields);
                }
            }
            syn::Item::Struct(model) => inspect_struct(model, file, fields),
            _ => {}
        }
    }
}

fn inspect_struct(model: &syn::ItemStruct, file: &str, fields: &mut Vec<PersonalField>) {
    let derived = derives(&model.attrs);
    let is_model = derived
        .iter()
        .any(|name| matches!(name.as_str(), "PersonalData" | "Orm" | "TursoModel"));
    let syn::Fields::Named(named) = &model.fields else {
        return;
    };
    for field in &named.named {
        let Some(ident) = &field.ident else {
            continue;
        };
        let name = ident.to_string();
        let markers = markers(field);
        let classification = if markers.privacy {
            "personal"
        } else if markers.encrypted {
            "encrypted"
        } else if markers.masked {
            "masked"
        } else if is_model && likely_personal(&name) {
            "review"
        } else {
            continue;
        };
        fields.push(PersonalField {
            model: model.ident.to_string(),
            field: name,
            file: crate::ui::error_report::sanitize(file),
            line: ident.span().start().line,
            classification: classification.to_string(),
            encrypted: markers.encrypted,
        });
    }
}

/// The inventory over the production sources. A file that does not parse is
/// counted in the detail; the inventory is never a pass/fail result.
pub(super) fn inventory(sources: &ProjectSources) -> PersonalData {
    if !sources.available {
        return PersonalData {
            status: EvidenceStatus::NotChecked("no src directory was available to inspect"),
            fields: Vec::new(),
            detail: "no src directory was available to inspect".to_string(),
        };
    }
    let mut fields = Vec::new();
    let mut unparsed = 0usize;
    for file in &sources.files {
        match syn::parse_file(&file.production) {
            Ok(parsed) => visit(&parsed.items, &file.display, &mut fields),
            Err(_) => unparsed += 1,
        }
    }
    let review = fields
        .iter()
        .filter(|field| field.classification == "review")
        .count();
    let detail = format!(
        "{} classified or flagged field(s); {review} need review. `review` is a field-name heuristic (email, phone, cpf, cnpj, ssn, birth, address, document, ip) on ORM models without #[privacy], #[orm(encrypted)] or #[orm(masked)]; it neither proves nor rules out personal data.{}",
        fields.len(),
        if unparsed == 0 {
            String::new()
        } else {
            format!(" {unparsed} source file(s) could not be parsed and were not inventoried.")
        },
    );
    PersonalData {
        status: EvidenceStatus::Observed(fields.len()),
        fields,
        detail,
    }
}

#[cfg(test)]
#[path = "personal_data_tests.rs"]
mod tests;

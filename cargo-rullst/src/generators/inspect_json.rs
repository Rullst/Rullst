//! `inspect models --json` and `inspect schema --json`: the structural views
//! of a project as one versioned JSON document each. Both read source files
//! only (no compilation, no database) and never follow a symlink.

use quote::ToTokens as _;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

pub(crate) const MODELS_SCHEMA: &str = "rullst.cli-models.v1";
pub(crate) const SCHEMA_SCHEMA: &str = "rullst.cli-schema.v1";
/// Upper bound for one model file; a larger file is reported, not parsed.
const MAX_MODEL_FILE_BYTES: u64 = 1024 * 1024;

/// A Rust type as written, without the token printer's padding
/// (`Option < String >` becomes `Option<String>`).
fn type_text(ty: &syn::Type) -> String {
    ty.to_token_stream()
        .to_string()
        .replace(" < ", "<")
        .replace(" >", ">")
        .replace(" , ", ", ")
        .replace(" :: ", "::")
        .replace("& ", "&")
}

fn items_of(source: &str) -> Option<Vec<Value>> {
    let file = syn::parse_file(source).ok()?;
    let mut items = Vec::new();
    for item in file.items {
        match item {
            syn::Item::Struct(item) if matches!(item.vis, syn::Visibility::Public(_)) => {
                let fields: Vec<Value> = match &item.fields {
                    syn::Fields::Named(named) => named
                        .named
                        .iter()
                        .filter(|field| matches!(field.vis, syn::Visibility::Public(_)))
                        .filter_map(|field| {
                            let name = field.ident.as_ref()?.to_string();
                            Some(json!({"name": name, "type": type_text(&field.ty)}))
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                items.push(json!({
                    "kind": "struct", "name": item.ident.to_string(),
                    "fields": fields, "variants": [],
                }));
            }
            syn::Item::Enum(item) if matches!(item.vis, syn::Visibility::Public(_)) => {
                let variants: Vec<String> = item
                    .variants
                    .iter()
                    .map(|variant| variant.ident.to_string())
                    .collect();
                items.push(json!({
                    "kind": "enum", "name": item.ident.to_string(),
                    "fields": [], "variants": variants,
                }));
            }
            _ => {}
        }
    }
    Some(items)
}

/// The `rullst.cli-models.v1` document for the `.rs` files directly under
/// `models_dir` (`mod.rs` excluded), in file-name order.
pub(crate) fn models_document(models_dir: &Path) -> Value {
    let mut files = Vec::new();
    let mut problems: Vec<String> = Vec::new();
    match fs::read_dir(models_dir) {
        Err(_) => problems.push(format!("{} directory not found", models_dir.display())),
        Ok(entries) => {
            let mut paths: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
            paths.sort();
            for path in paths {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let regular = fs::symlink_metadata(&path)
                    .map(|meta| meta.is_file() && meta.len() <= MAX_MODEL_FILE_BYTES)
                    .unwrap_or(false);
                if path.extension().is_none_or(|ext| ext != "rs") || name == "mod.rs" {
                    continue;
                }
                match fs::read_to_string(&path).ok().filter(|_| regular) {
                    Some(source) => match items_of(&source) {
                        Some(items) => files.push(json!({"file": name, "items": items})),
                        None => problems.push(format!("{name} could not be parsed as Rust")),
                    },
                    None => problems.push(format!("{name} is not a readable regular file")),
                }
            }
        }
    }
    json!({
        "schema": MODELS_SCHEMA,
        "complete": problems.is_empty(),
        "incomplete_reason": if problems.is_empty() { Value::Null } else { json!(problems.join("; ")) },
        "files": files,
    })
}

/// The `rullst.cli-schema.v1` document: the project-provided snapshot at
/// `snapshot` when present (embedded as parsed), else `derived` (the models
/// extracted from `#[derive(Orm)]` structs).
pub(crate) fn schema_document(
    snapshot: &Path,
    derived: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut document = if snapshot.is_file() {
        let value: Value = serde_json::from_str(&fs::read_to_string(snapshot)?)?;
        json!({"source": "rullst-schema.json", "snapshot": value})
    } else {
        derived
    };
    if let Some(object) = document.as_object_mut() {
        object.insert("schema".to_owned(), json!(SCHEMA_SCHEMA));
    }
    Ok(document)
}

#[cfg(test)]
#[path = "inspect_json_tests.rs"]
mod tests;

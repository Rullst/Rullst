// src/generators/mod.rs — Shared helpers and generator modules definition.

use std::fs;
use std::path::Path;

pub mod academy_doctor;
pub(crate) mod add;
pub(crate) mod age_gate;
pub mod ai_context;
pub(crate) mod api_contract;
pub mod audit;
mod audit_compliance;
mod audit_evidence;
mod audit_idor;
mod audit_purl;
pub(crate) mod audit_report;
mod audit_scope;
mod audit_source;
pub mod auth;
pub mod billing;
pub mod build;
pub mod chat;
mod consumer_files;
mod consumer_support;
pub mod controller;
pub mod cors_jwt;
pub mod db;
pub mod deploy;
pub(crate) mod deploy_doctor;
pub mod desktop;
pub mod dev;
pub mod diagram;
pub mod doctor;
pub mod eject;
pub(crate) mod footprint;
pub mod foundry;
pub mod grpc;
pub mod hook;
pub mod inspect;
pub mod introspect;
pub mod iot;
pub mod island;
pub mod k8s;
pub mod live;
pub mod mail;
pub mod middleware;
pub mod migration;
pub mod model;
pub mod openapi;
pub(crate) mod output_guard;
pub(crate) mod platform_name;
pub(crate) mod privacy;
pub mod project;
pub mod resource;
pub mod scalar;
pub mod schema_diff;
mod source_walk;
pub mod ts;
pub mod worker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectOrmBackend {
    Sqlx,
    Turso,
}

/// Verifies if the current execution directory is a valid Rullst project
pub fn is_rullst_project() -> bool {
    let cargo_toml_path = Path::new("Cargo.toml");
    if !cargo_toml_path.exists() {
        return false;
    }
    match fs::read_to_string(cargo_toml_path) {
        Ok(content) => content.contains("rullst"),
        Err(_) => false,
    }
}

pub(crate) fn project_orm_backend() -> ProjectOrmBackend {
    let Ok(manifest) = fs::read_to_string("Cargo.toml") else {
        return ProjectOrmBackend::Sqlx;
    };
    project_orm_backend_from_manifest(&manifest)
}

fn project_orm_backend_from_manifest(manifest: &str) -> ProjectOrmBackend {
    let Ok(manifest) = toml::from_str::<toml::Value>(manifest) else {
        return ProjectOrmBackend::Sqlx;
    };
    let Some(dependencies) = manifest.get("dependencies").and_then(toml::Value::as_table) else {
        return ProjectOrmBackend::Sqlx;
    };
    let has_sqlx = dependencies.contains_key("sqlx");
    let has_turso = ["rullst", "rullst-orm"].into_iter().any(|name| {
        dependencies
            .get(name)
            .and_then(toml::Value::as_table)
            .and_then(|dependency| dependency.get("features"))
            .and_then(toml::Value::as_array)
            .is_some_and(|features| {
                features.iter().any(|feature| {
                    feature
                        .as_str()
                        .is_some_and(|feature| matches!(feature, "orm-turso" | "turso"))
                })
            })
    });

    if has_turso && !has_sqlx {
        ProjectOrmBackend::Turso
    } else {
        ProjectOrmBackend::Sqlx
    }
}

/// Returns whether a generated module/type token is a non-keyword Rust identifier.
pub(crate) fn is_valid_rust_identifier(value: &str) -> bool {
    !value.is_empty() && syn::parse_str::<syn::Ident>(value).is_ok()
}

/// AST-based module registration for registering new submodules in mod.rs or main.rs
pub fn register_mod_ast(
    mod_path: &Path,
    module_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !mod_path.exists() {
        fs::write(mod_path, "")?;
    }

    let content = fs::read_to_string(mod_path)?;
    if let Ok(file_ast) = syn::parse_file(&content) {
        let already_registered = file_ast.items.iter().any(|item| {
            if let syn::Item::Mod(item_mod) = item {
                item_mod.ident == module_name
            } else {
                false
            }
        });

        if already_registered {
            return Ok(());
        }
    }

    let decl = format!("pub mod {};\n", module_name);
    let mut new_content = content;
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(&decl);
    fs::write(mod_path, new_content)?;

    Ok(())
}

/// Normalizes the controller name to snake_case with the "_controller" suffix
pub fn to_snake_case(s: &str) -> String {
    let mut base = s.to_string();
    // Remove the case-insensitive suffix if it already exists
    if base.to_lowercase().ends_with("controller") {
        let len = base.len();
        base.truncate(len - 10);
    }

    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in base.chars() {
        if c == '_' || c == '-' {
            result.push('_');
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = true;
        }
    }

    result.push_str("_controller");

    // Limpa possíveis underscores repetidos (ex: users__controller)
    let mut clean_result = String::new();
    let mut prev_is_underscore = false;
    for c in result.chars() {
        if c == '_' {
            if !prev_is_underscore {
                clean_result.push(c);
            }
            prev_is_underscore = true;
        } else {
            clean_result.push(c);
            prev_is_underscore = false;
        }
    }
    clean_result
}

/// Converts the controller name to CamelCase (PascalCase) with the "Controller" suffix
pub fn to_camel_case(s: &str) -> String {
    let snake = to_snake_case(s);
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in snake.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Normalizes the model name to snake_case
pub fn model_to_snake_case(s: &str) -> String {
    let mut base = s.to_string();
    // Remove the "Model" or "model" suffix if present
    if base.to_lowercase().ends_with("model") {
        let len = base.len();
        base.truncate(len - 5);
    }

    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in base.chars() {
        if c == '_' || c == '-' {
            result.push('_');
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = true;
        }
    }

    // Limpa underscores repetidos
    let mut clean_result = String::new();
    let mut prev_is_underscore = false;
    for c in result.chars() {
        if c == '_' {
            if !prev_is_underscore {
                clean_result.push(c);
            }
            prev_is_underscore = true;
        } else {
            clean_result.push(c);
            prev_is_underscore = false;
        }
    }
    clean_result.trim_matches('_').to_string()
}

/// Converts the model name to PascalCase (CamelCase)
pub fn model_to_pascal_case(s: &str) -> String {
    let snake = model_to_snake_case(s);
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in snake.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Pluralizes the table name following the Active Record convention
pub fn pluralize(s: &str) -> String {
    let lower = s.to_lowercase();
    if lower.ends_with("ss") {
        format!("{}es", lower)
    } else if lower.ends_with("s") {
        lower
    } else if lower.ends_with("y") {
        let len = lower.len();
        if len > 1 {
            let before_y = &lower[len - 2..len - 1];
            if before_y == "a"
                || before_y == "e"
                || before_y == "i"
                || before_y == "o"
                || before_y == "u"
            {
                format!("{}s", lower)
            } else {
                format!("{}ies", &lower[..len - 1])
            }
        } else {
            format!("{}s", lower)
        }
    } else if lower.ends_with("ch")
        || lower.ends_with("sh")
        || lower.ends_with("x")
        || lower.ends_with("z")
    {
        format!("{}es", lower)
    } else {
        format!("{}s", lower)
    }
}

/// Normalizes the middleware name to snake_case with the "_middleware" suffix
pub fn middleware_to_snake_case(s: &str) -> String {
    let mut base = s.to_string();
    // Remove the case-insensitive suffix if it already exists
    if base.to_lowercase().ends_with("middleware") {
        let len = base.len();
        base.truncate(len - 10);
    }

    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in base.chars() {
        if c == '_' || c == '-' {
            result.push('_');
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = true;
        }
    }

    result.push_str("_middleware");

    // Clean up potential duplicate underscores (e.g., auth__middleware)
    let mut clean_result = String::new();
    let mut prev_is_underscore = false;
    for c in result.chars() {
        if c == '_' {
            if !prev_is_underscore {
                clean_result.push(c);
            }
            prev_is_underscore = true;
        } else {
            clean_result.push(c);
            prev_is_underscore = false;
        }
    }
    clean_result.trim_matches('_').to_string()
}

/// Locates a TOML parse failure as `line L, column C`, without its message.
/// The `toml` error text quotes the offending source line, which can hold a
/// secret such as `app_key` or a database URL, so it is never reported.
pub(crate) fn toml_error_position(content: &str, error: &toml::de::Error) -> String {
    let Some(span) = error.span() else {
        return "an unknown position".to_owned();
    };
    let before = content.get(..span.start).unwrap_or(content);
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |text| text.chars().count())
        + 1;
    format!("line {line}, column {column}")
}

#[cfg(test)]
mod tests;

// src/generators/mod.rs — Shared helpers and generator modules definition.

use std::fs;
use std::path::Path;

pub mod academy_doctor;
pub mod ai_context;
pub mod audit;
mod audit_compliance;
mod audit_evidence;
pub mod auth;
pub mod billing;
pub mod build;
pub mod chat;
pub mod controller;
pub mod cors_jwt;
pub mod db;
pub mod deploy;
pub mod desktop;
pub mod dev;
pub mod diagram;
pub mod doctor;
pub mod eject;
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
pub mod project;
pub mod resource;
pub mod scalar;
pub mod schema_diff;
pub mod ts;
pub mod worker;

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

#[cfg(test)]
mod tests {
    use super::*;
    use syn::visit::Visit;

    const PANICKING_RUNTIME_CALLS: [&str; 5] = [
        ".unwrap(",
        ".expect(",
        "panic!(",
        "todo!(",
        "unimplemented!(",
    ];

    #[derive(Default)]
    struct RuntimeLiteralAudit {
        violations: Vec<String>,
    }

    impl<'ast> Visit<'ast> for RuntimeLiteralAudit {
        fn visit_lit_str(&mut self, literal: &'ast syn::LitStr) {
            let value = literal.value();
            for needle in PANICKING_RUNTIME_CALLS {
                // Unit tests use the exact needles to assert the generated
                // output. Longer literals containing them are emitted source
                // candidates and must remain panic-free.
                if value != needle && value.contains(needle) {
                    self.violations.push(value.clone());
                }
            }
            syn::visit::visit_lit_str(self, literal);
        }
    }

    #[test]
    fn test_is_rullst_project() {
        // Since we are running in the Rullst repo with a Cargo.toml containing "rullst", this should return true
        assert!(is_rullst_project());
    }

    #[test]
    fn test_register_mod_ast_flow() {
        let temp_dir =
            std::env::temp_dir().join(format!("rullst_test_mod_{}", rand::random::<u64>()));
        let _ = fs::create_dir_all(&temp_dir);
        let mod_file = temp_dir.join("mod.rs");

        // 1. Register in new non-existent file
        register_mod_ast(&mod_file, "users").unwrap();
        let content1 = fs::read_to_string(&mod_file).unwrap();
        assert!(content1.contains("pub mod users;"));

        // 2. Register same module again (idempotent, shouldn't duplicate)
        register_mod_ast(&mod_file, "users").unwrap();
        let content2 = fs::read_to_string(&mod_file).unwrap();
        assert_eq!(content1, content2);

        // 3. Register a second module
        register_mod_ast(&mod_file, "posts").unwrap();
        let content3 = fs::read_to_string(&mod_file).unwrap();
        assert!(content3.contains("pub mod users;"));
        assert!(content3.contains("pub mod posts;"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn generated_identifiers_reject_keywords_and_syntax() {
        assert!(is_valid_rust_identifier("billing_customer"));
        assert!(is_valid_rust_identifier("BillingCustomer"));
        assert!(!is_valid_rust_identifier("type"));
        assert!(!is_valid_rust_identifier("bad-name"));
        assert!(!is_valid_rust_identifier(""));
    }

    #[test]
    fn embedded_runtime_templates_have_no_panicking_calls() {
        let generators_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generators");
        let mut violations = Vec::new();

        for entry in walkdir::WalkDir::new(generators_dir) {
            let entry = entry.expect("generator source entry");
            let path = entry.path();
            if !entry.file_type().is_file()
                || path.extension().and_then(std::ffi::OsStr::to_str) != Some("rs")
            {
                continue;
            }

            let source = fs::read_to_string(path).expect("generator source");
            let syntax = syn::parse_file(&source).expect("generator source must parse");
            let mut audit = RuntimeLiteralAudit::default();
            audit.visit_file(&syntax);

            for literal in audit.violations {
                violations.push(format!("{}: {literal:?}", path.display()));
            }
        }

        assert!(
            violations.is_empty(),
            "embedded generated-runtime literals contain panic paths:\n{}",
            violations.join("\n")
        );
    }
}

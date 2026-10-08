use std::{fs, path::Path};

use crate::blueprints::{BLANK_BLUEPRINT_ID, LMS_BLUEPRINT_ID, SAAS_BLUEPRINT_ID};
use crate::generators::project::PolyglotIntegration;

fn is_matching_local_package(path: &Path, crate_name: &str, crate_version: &str) -> bool {
    let Ok(contents) = fs::read_to_string(path.join("Cargo.toml")) else {
        return false;
    };
    let Ok(manifest) = toml::from_str::<toml::Value>(&contents) else {
        return false;
    };

    let Some(package) = manifest.get("package").and_then(toml::Value::as_table) else {
        return false;
    };
    package.get("name").and_then(toml::Value::as_str) == Some(crate_name)
        && package.get("version").and_then(toml::Value::as_str) == Some(crate_version)
}

fn dependency_source(
    current_dir: &Path,
    crate_name: &str,
    crate_version: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let sibling = current_dir.join(crate_name);
    let source_checkout = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|root| root.join(crate_name));
    let invocation_is_matching_checkout = is_matching_local_package(
        &current_dir.join("cargo-rullst"),
        "cargo-rullst",
        env!("CARGO_PKG_VERSION"),
    );
    let local_path = (invocation_is_matching_checkout
        && is_matching_local_package(&sibling, crate_name, crate_version))
    .then_some(sibling)
    .or_else(|| {
        (crate_version == env!("CARGO_PKG_VERSION") && crate_version.contains('-'))
            .then_some(source_checkout)
            .flatten()
            .filter(|path| is_matching_local_package(path, crate_name, crate_version))
    });

    if let Some(local_path) = local_path {
        let absolute_path = local_path
            .canonicalize()?
            .display()
            .to_string()
            .replace(r"\\?\", "")
            .replace('\\', "/");
        let path_literal = toml_edit::value(absolute_path).to_string();
        Ok(format!("path = {path_literal}"))
    } else {
        Ok(format!("version = \"{crate_version}\""))
    }
}

fn dependency_line(
    current_dir: &Path,
    crate_name: &str,
    crate_version: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let source = dependency_source(current_dir, crate_name, crate_version)?;
    Ok(format!("{crate_name} = {{ {source} }}\n"))
}

#[allow(clippy::too_many_arguments)]
pub fn build_cargo_toml(
    package_name: &str,
    hot_reload: bool,
    db_needed: bool,
    db_provider: &str,
    polyglot_integrations: &[PolyglotIntegration],
    wants_ai: bool,
    wants_redis: bool,
    blueprint_selection: usize,
    frontend_engine: &str,
    current_dir: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut cargo_toml = String::new();

    let crate_version = env!("CARGO_PKG_VERSION");
    let rullst_source = dependency_source(current_dir, "rullst", crate_version)?;
    let rullst_dep = format!("rullst = {{ {rullst_source}, default-features = false");

    let mut rullst_features = Vec::new();
    if db_needed {
        rullst_features.push("orm");
        if let Some(profile) = relational_profile(db_provider) {
            rullst_features.push(profile);
        }
    }
    if wants_ai {
        rullst_features.push("ai");
    }
    if wants_redis {
        rullst_features.push("redis");
    }
    for integration in polyglot_integrations {
        rullst_features.push(integration.rullst_feature());
    }

    rullst_features.push("studio");
    if blueprint_selection != BLANK_BLUEPRINT_ID {
        rullst_features.push("nexus");
    }
    if matches!(blueprint_selection, LMS_BLUEPRINT_ID | SAAS_BLUEPRINT_ID) {
        rullst_features.push("auth");
    }
    if blueprint_selection == SAAS_BLUEPRINT_ID {
        rullst_features.push("capital");
    }

    let rullst_line = if rullst_features.is_empty() {
        " }".to_string()
    } else {
        let feats_str = rullst_features
            .iter()
            .map(|f| format!("\"{}\"", f))
            .collect::<Vec<_>>()
            .join(", ");
        format!(", features = [{}] }}", feats_str)
    };

    if hot_reload {
        cargo_toml.push_str(&format!(
            r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2024"
rust-version = "1.96.0"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
"#
        ));
    } else {
        cargo_toml.push_str(&format!(
            r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2024"
rust-version = "1.96.0"

[dependencies]
"#
        ));
    }

    cargo_toml.push_str(&rullst_dep);
    cargo_toml.push_str(&rullst_line);
    cargo_toml.push('\n');
    cargo_toml.push_str("serde = { version = \"1.0\", features = [\"derive\"] }\n");
    cargo_toml.push_str("serde_json = \"1.0\"\n");
    cargo_toml.push_str("tokio = { version = \"1.0\", features = [\"full\"] }\n");
    cargo_toml.push_str("tracing = \"0.1\"\n");
    cargo_toml.push_str("tracing-subscriber = \"0.3\"\n");

    if db_needed || wants_redis || !polyglot_integrations.is_empty() {
        let mut orm_features = polyglot_integrations
            .iter()
            .map(|integration| format!("\"{}\"", integration.orm_feature()))
            .collect::<Vec<_>>();
        if wants_redis {
            orm_features.push("\"redis\"".to_owned());
        }
        if db_needed && let Some(profile) = relational_profile(db_provider) {
            orm_features.push(format!("\"{profile}\""));
        }
        let orm_features = orm_features.join(", ");
        let orm_source = dependency_source(current_dir, "rullst-orm", crate_version)?;
        // rullst-orm's default `drivers-all` would compile the bundled SQLite,
        // PostgreSQL and MySQL drivers into every strict single-backend profile.
        let orm_defaults = if db_needed && relational_profile(db_provider).is_some() {
            ", default-features = false"
        } else {
            ""
        };
        if orm_features.is_empty() {
            cargo_toml.push_str(&format!("rullst-orm = {{ {orm_source}{orm_defaults} }}\n"));
        } else {
            cargo_toml.push_str(&format!(
                "rullst-orm = {{ {orm_source}{orm_defaults}, features = [{orm_features}] }}\n"
            ));
        }
    }

    if db_needed && db_provider != "Turso" {
        let sqlx_driver_feature = match db_provider {
            "Postgres" => "postgres",
            "MySQL" | "MariaDB" => "mysql",
            _ => "sqlite",
        };
        let sqlx_features = format!(
            "\"runtime-tokio\", \"tls-rustls\", \"{}\"",
            sqlx_driver_feature
        );

        cargo_toml.push_str(&format!(
            r#"sqlx = {{ version = "0.9", default-features = false, features = [{sqlx_features}] }}
"#,
            sqlx_features = sqlx_features
        ));
    }

    if db_provider == "Turso" {
        cargo_toml.push_str("dotenvy = \"0.15\"\n");
    }

    if matches!(blueprint_selection, LMS_BLUEPRINT_ID | SAAS_BLUEPRINT_ID) {
        let auth_dep = dependency_line(current_dir, "rullst-auth", crate_version)?;
        cargo_toml.push_str(&auth_dep);
    }

    if blueprint_selection == SAAS_BLUEPRINT_ID {
        let capital_dep = dependency_line(current_dir, "rullst-capital", crate_version)?;
        cargo_toml.push_str(&capital_dep);

        let connect_dep = dependency_line(current_dir, "rullst-connect", crate_version)?;
        cargo_toml.push_str(&connect_dep);
    }

    let security_dep = dependency_line(current_dir, "rullst-security", crate_version)?;
    cargo_toml.push_str(&security_dep);

    let fe_dep = crate::blueprints::common::frontend_cargo_dependency(frontend_engine);
    cargo_toml.push_str(&fe_dep);

    cargo_toml.push_str(
        r#"
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["Document", "Element", "EventTarget", "Window"] }

[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(feature, values("redis"))'] }
"#,
    );
    cargo_toml.push_str(RELEASE_PROFILE);
    cargo_toml.push_str("\n[workspace]\n");

    Ok(cargo_toml)
}

/// The `[profile.release]` of every generated manifest (see its comments).
pub(crate) const RELEASE_PROFILE: &str = r#"
# Smaller release binaries. Debug builds keep Cargo's defaults. `panic` stays
# "unwind" on purpose: a panicking handler must not take the server down.
# `strip = "symbols"` removes symbol names from release backtraces; use
# `strip = "debuginfo"` when you need symbolized production backtraces.
[profile.release]
lto = "thin"
codegen-units = 1
strip = "symbols"
"#;

fn relational_profile(db_provider: &str) -> Option<&'static str> {
    match db_provider {
        "Postgres" => Some("strict-postgres"),
        "MySQL" | "MariaDB" => Some("strict-mysql"),
        "Sqlite" => Some("strict-sqlite"),
        _ => None,
    }
}

#[cfg(test)]
#[path = "cargo_toml_tests.rs"]
mod tests;

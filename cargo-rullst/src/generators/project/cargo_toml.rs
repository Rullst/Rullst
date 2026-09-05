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
    if blueprint_selection != BLANK_BLUEPRINT_ID || db_needed {
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
edition = "2021"

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
edition = "2021"

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
        if orm_features.is_empty() {
            cargo_toml.push_str(&format!("rullst-orm = {{ {orm_source} }}\n"));
        } else {
            cargo_toml.push_str(&format!(
                "rullst-orm = {{ {orm_source}, features = [{orm_features}] }}\n"
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

[workspace]
"#,
    );

    Ok(cargo_toml)
}

fn relational_profile(db_provider: &str) -> Option<&'static str> {
    match db_provider {
        "Postgres" => Some("strict-postgres"),
        "MySQL" | "MariaDB" => Some("strict-mysql"),
        "Sqlite" => Some("strict-sqlite"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprints::{BLOG_BLUEPRINT_ID, LMS_BLUEPRINT_ID, SAAS_BLUEPRINT_ID};

    fn isolated_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("rullst-manifest-{}", rand::random::<u64>()))
    }

    #[test]
    fn package_name_is_distinct_from_the_rust_module_name() {
        let manifest = build_cargo_toml(
            "dummy-test",
            false,
            false,
            "Sqlite",
            &[],
            false,
            false,
            BLANK_BLUEPRINT_ID,
            "Zero-Bundle HTMX",
            &isolated_root(),
        )
        .expect("Cargo manifest");
        assert!(manifest.contains("name = \"dummy-test\""));
    }

    #[test]
    fn stable_blueprint_ids_enable_only_their_required_domain_features() {
        let saas = build_cargo_toml(
            "saas",
            false,
            true,
            "Sqlite",
            &[],
            false,
            false,
            SAAS_BLUEPRINT_ID,
            "Zero-Bundle HTMX",
            &isolated_root(),
        )
        .expect("SaaS manifest");
        let blog = build_cargo_toml(
            "blog",
            false,
            true,
            "Sqlite",
            &[],
            false,
            false,
            BLOG_BLUEPRINT_ID,
            "Zero-Bundle HTMX",
            &isolated_root(),
        )
        .expect("Blog manifest");
        let lms = build_cargo_toml(
            "lms",
            false,
            true,
            "Sqlite",
            &[],
            false,
            false,
            LMS_BLUEPRINT_ID,
            "Zero-Bundle HTMX",
            &isolated_root(),
        )
        .expect("LMS manifest");
        assert!(saas.contains("\"auth\""));
        assert!(saas.contains("\"capital\""));
        assert!(lms.contains("\"auth\""));
        assert!(!lms.contains("\"capital\""));
        assert!(!blog.contains("\"auth\""));
        assert!(!blog.contains("\"capital\""));
    }

    #[test]
    fn registry_dependencies_preserve_the_cli_prerelease_version() {
        let root = isolated_root();
        for crate_name in [
            "rullst",
            "rullst-orm",
            "rullst-auth",
            "rullst-capital",
            "rullst-connect",
            "rullst-security",
        ] {
            let dependency =
                dependency_line(&root, crate_name, "12.0.0-rc.7").expect("registry dependency");
            assert_eq!(
                dependency,
                format!("{crate_name} = {{ version = \"12.0.0-rc.7\" }}\n")
            );
        }
    }

    #[test]
    fn arbitrary_matching_sibling_is_not_trusted_as_a_framework_checkout() {
        let root = tempfile::tempdir().expect("isolated invocation directory");
        let sibling = root.path().join("rullst");
        fs::create_dir(&sibling).expect("lookalike crate directory");
        fs::write(
            sibling.join("Cargo.toml"),
            "[package]\nname = \"rullst\"\nversion = \"12.0.0-rc.7\"\n",
        )
        .expect("lookalike manifest");
        assert_eq!(
            dependency_source(root.path(), "rullst", "12.0.0-rc.7").expect("registry fallback"),
            "version = \"12.0.0-rc.7\""
        );
    }

    #[test]
    fn source_checkout_is_available_to_the_current_prerelease() {
        let source = dependency_source(Path::new("/tmp"), "rullst", env!("CARGO_PKG_VERSION"))
            .expect("current dependency source");
        assert!(source.starts_with("path = "), "unexpected source: {source}");
    }

    #[test]
    fn generated_tera_dependency_remains_available_offline() {
        let engine = tera::Tera::default();
        assert_eq!(engine.get_template_names().count(), 0);
        assert_eq!(
            crate::blueprints::common::frontend_cargo_dependency("Tera Templates"),
            "tera = \"2.2\"\n"
        );
    }

    #[test]
    fn selected_persistence_integrations_enable_only_their_features() {
        let manifest = build_cargo_toml(
            "polyglot-app",
            false,
            true,
            "MariaDB",
            &[
                PolyglotIntegration::Turso,
                PolyglotIntegration::MongoDb,
                PolyglotIntegration::DuckDb,
                PolyglotIntegration::SurrealDb,
                PolyglotIntegration::Qdrant,
            ],
            false,
            false,
            BLANK_BLUEPRINT_ID,
            "Zero-Bundle HTMX",
            &isolated_root(),
        )
        .expect("polyglot manifest");
        for feature in [
            "orm-turso",
            "orm-mongodb",
            "orm-duckdb",
            "orm-surrealdb",
            "orm-qdrant",
            "turso",
            "mongodb",
            "duckdb",
            "surrealdb",
            "qdrant",
        ] {
            assert!(manifest.contains(&format!("\"{feature}\"")));
        }
        assert!(manifest.contains("\"mysql\""));
    }

    #[test]
    fn turso_primary_uses_hrana_features_without_a_direct_sqlx_driver() {
        let manifest = build_cargo_toml(
            "edge-primary",
            false,
            true,
            "Turso",
            &[PolyglotIntegration::Turso],
            false,
            false,
            BLANK_BLUEPRINT_ID,
            "Zero-Bundle HTMX",
            &isolated_root(),
        )
        .expect("Turso-primary manifest");

        assert!(manifest.contains("\"orm-turso\""));
        assert!(manifest.contains("features = [\"turso\"]"));
        assert!(manifest.contains("dotenvy = \"0.15\""));
        assert!(!manifest.lines().any(|line| line.starts_with("sqlx = ")));
    }

    #[test]
    fn primary_relational_choice_selects_one_strict_profile() {
        for (provider, expected, rejected) in [
            (
                "Sqlite",
                "strict-sqlite",
                ["strict-postgres", "strict-mysql"],
            ),
            (
                "Postgres",
                "strict-postgres",
                ["strict-sqlite", "strict-mysql"],
            ),
            (
                "MySQL",
                "strict-mysql",
                ["strict-sqlite", "strict-postgres"],
            ),
            (
                "MariaDB",
                "strict-mysql",
                ["strict-sqlite", "strict-postgres"],
            ),
        ] {
            let manifest = build_cargo_toml(
                "strict-app",
                false,
                true,
                provider,
                &[],
                false,
                false,
                BLANK_BLUEPRINT_ID,
                "Zero-Bundle HTMX",
                &isolated_root(),
            )
            .expect("strict database manifest");
            let parsed: toml::Value = toml::from_str(&manifest).expect("valid Cargo manifest");
            for dependency in ["rullst", "rullst-orm"] {
                let features = parsed["dependencies"][dependency]["features"]
                    .as_array()
                    .expect("generated dependency feature array")
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .collect::<Vec<_>>();
                assert!(
                    features.contains(&expected),
                    "{provider}:{dependency} missing {expected}"
                );
                for other in rejected {
                    assert!(
                        !features.contains(&other),
                        "{provider}:{dependency} unexpectedly selected {other}"
                    );
                }
            }
            assert_eq!(
                parsed["dependencies"]["rullst"]["default-features"].as_bool(),
                Some(false),
                "{provider}: generated applications must not re-enable the umbrella default database profile"
            );
        }
    }
}

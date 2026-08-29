// cargo-rullst/src/generators/project/cargo_toml.rs — Cargo.toml generator (< 300 lines).

use std::path::Path;

use crate::blueprints::{BLANK_BLUEPRINT_ID, LMS_BLUEPRINT_ID, SAAS_BLUEPRINT_ID};

fn dependency_source(
    current_dir: &Path,
    crate_name: &str,
    crate_version: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let sibling = current_dir.join(crate_name);
    if sibling.exists() {
        let absolute_path = sibling
            .canonicalize()?
            .display()
            .to_string()
            .replace(r"\\?\", "")
            .replace('\\', "/");
        Ok(format!("path = \"{absolute_path}\""))
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
    wants_ai: bool,
    wants_redis: bool,
    blueprint_selection: usize,
    frontend_engine: &str,
    current_dir: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut cargo_toml = String::new();

    let crate_version = env!("CARGO_PKG_VERSION");
    let rullst_source = dependency_source(current_dir, "rullst", crate_version)?;
    let rullst_dep = format!("rullst = {{ {rullst_source}");

    let mut rullst_features = Vec::new();
    if db_needed {
        rullst_features.push("orm");
    }
    if wants_ai {
        rullst_features.push("ai");
    }
    if wants_redis {
        rullst_features.push("redis");
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

    if db_needed {
        let sqlx_driver_feature = match db_provider {
            "Postgres" => "postgres",
            "MySQL" => "mysql",
            _ => "sqlite",
        };
        let sqlx_features = format!(
            "\"runtime-tokio\", \"tls-rustls\", \"{}\"",
            sqlx_driver_feature
        );

        let orm_dep = dependency_line(current_dir, "rullst-orm", crate_version)?;

        cargo_toml.push_str(&format!(
            r#"{orm_dep}sqlx = {{ version = "0.9", default-features = false, features = [{sqlx_features}] }}
"#,
            orm_dep = orm_dep,
            sqlx_features = sqlx_features
        ));
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
    fn generated_tera_dependency_remains_available_offline() {
        let engine = tera::Tera::default();
        assert_eq!(engine.get_template_names().count(), 0);
        assert_eq!(
            crate::blueprints::common::frontend_cargo_dependency("Tera Templates"),
            "tera = \"2.2\"\n"
        );
    }
}

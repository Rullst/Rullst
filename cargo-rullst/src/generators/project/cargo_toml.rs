// cargo-rullst/src/generators/project/cargo_toml.rs — Cargo.toml generator (< 300 lines).

use std::path::Path;

use crate::blueprints::{BLANK_BLUEPRINT_ID, SAAS_BLUEPRINT_ID};

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
    let sibling_rullst = current_dir.join("rullst");
    let rullst_dep = if sibling_rullst.exists() {
        let absolute_path = sibling_rullst
            .canonicalize()?
            .display()
            .to_string()
            .replace(r"\\?\", "")
            .replace("\\", "/");
        format!("rullst = {{ path = \"{}\"", absolute_path)
    } else {
        format!("rullst = {{ version = \"{}\"", crate_version)
    };

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
    if blueprint_selection == SAAS_BLUEPRINT_ID {
        rullst_features.push("auth");
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

        let sibling_orm = current_dir.join("rullst-orm");
        let orm_dep = if sibling_orm.exists() {
            let absolute_path = sibling_orm
                .canonicalize()?
                .display()
                .to_string()
                .replace(r"\\?\", "")
                .replace("\\", "/");
            format!("rullst-orm = {{ path = \"{}\" }}\n", absolute_path)
        } else {
            "rullst-orm = \"12.0.0\"\n".to_string()
        };

        cargo_toml.push_str(&format!(
            r#"{orm_dep}sqlx = {{ version = "0.9", default-features = false, features = [{sqlx_features}] }}
"#,
            orm_dep = orm_dep,
            sqlx_features = sqlx_features
        ));
    }

    if blueprint_selection == SAAS_BLUEPRINT_ID {
        let sibling_auth = current_dir.join("rullst-auth");
        let auth_dep = if sibling_auth.exists() {
            let absolute_path = sibling_auth
                .canonicalize()?
                .display()
                .to_string()
                .replace(r"\\?\", "")
                .replace("\\", "/");
            format!("rullst-auth = {{ path = \"{}\" }}\n", absolute_path)
        } else {
            "rullst-auth = \"12.0.0\"\n".to_string()
        };
        cargo_toml.push_str(&auth_dep);

        let sibling_capital = current_dir.join("rullst-capital");
        let capital_dep = if sibling_capital.exists() {
            let absolute_path = sibling_capital
                .canonicalize()?
                .display()
                .to_string()
                .replace(r"\\?\", "")
                .replace("\\", "/");
            format!("rullst-capital = {{ path = \"{}\" }}\n", absolute_path)
        } else {
            "rullst-capital = \"12.0.0\"\n".to_string()
        };
        cargo_toml.push_str(&capital_dep);

        let sibling_path = current_dir.join("rullst-connect");
        let connect_dep = if sibling_path.exists() {
            let absolute_path = sibling_path
                .canonicalize()?
                .display()
                .to_string()
                .replace(r"\\?\", "")
                .replace("\\", "/");
            format!("rullst-connect = {{ path = \"{}\" }}\n", absolute_path)
        } else {
            "rullst-connect = \"12.0.0\"\n".to_string()
        };
        cargo_toml.push_str(&connect_dep);
    }

    let sibling_security = current_dir.join("rullst-security");
    let security_dep = if sibling_security.exists() {
        if let Ok(canon) = sibling_security.canonicalize() {
            let absolute_path = canon
                .display()
                .to_string()
                .replace(r"\\?\", "")
                .replace("\\", "/");
            format!("rullst-security = {{ path = \"{}\" }}\n", absolute_path)
        } else {
            "rullst-security = \"12.0.0\"\n".to_string()
        }
    } else {
        "rullst-security = \"12.0.0\"\n".to_string()
    };
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
    use crate::blueprints::{BLOG_BLUEPRINT_ID, SAAS_BLUEPRINT_ID};

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
    fn stable_saas_id_alone_enables_auth_and_capital() {
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
        assert!(saas.contains("\"auth\""));
        assert!(saas.contains("\"capital\""));
        assert!(!blog.contains("\"auth\""));
        assert!(!blog.contains("\"capital\""));
    }
}

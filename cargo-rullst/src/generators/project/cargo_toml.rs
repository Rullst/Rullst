// cargo-rullst/src/generators/project/cargo_toml.rs — Cargo.toml generator (< 300 lines).

use std::path::Path;

pub fn build_cargo_toml(
    project_name_safe: &str,
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
    if hot_reload {
        rullst_features.push("hot-reload");
    }
    if wants_ai {
        rullst_features.push("ai");
    }
    if wants_redis {
        rullst_features.push("redis");
    }

    if blueprint_selection == 1 {
        rullst_features.push("nexus");
        rullst_features.push("studio");
    }

    let rullst_line = if rullst_features.is_empty() {
        format!("{}}}", rullst_dep)
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
name = "{project_name_safe}"
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
name = "{project_name_safe}"
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
    cargo_toml.push_str("tokio = { version = \"1.0\", features = [\"full\"] }\n");
    cargo_toml.push_str("tracing = \"0.1\"\n");
    cargo_toml.push_str("tracing-subscriber = \"0.3\"\n");

    if db_needed {
        let sqlx_driver_feature = match db_provider {
            "Postgres" => "postgres",
            "MySQL" => "mysql",
            _ => "sqlite",
        };
        let sqlx_features = format!("\"runtime-tokio-rustls\", \"{}\"", sqlx_driver_feature);

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
            r#"{orm_dep}sqlx = {{ version = "0.8", default-features = false, features = [{sqlx_features}] }}
"#,
            orm_dep = orm_dep,
            sqlx_features = sqlx_features
        ));
    }

    if blueprint_selection == 3 {
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
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(feature, values("redis"))'] }

[workspace]
"#,
    );

    Ok(cargo_toml)
}

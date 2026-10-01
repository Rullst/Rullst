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
fn blank_database_project_does_not_compile_an_unmounted_nexus() {
    let blank = build_cargo_toml(
        "blank-db",
        false,
        true,
        "Sqlite",
        &[],
        false,
        false,
        BLANK_BLUEPRINT_ID,
        "Zero-Bundle HTMX",
        &isolated_root(),
    )
    .expect("Blank database manifest");

    assert!(blank.contains("\"studio\""));
    assert!(!blank.contains("\"nexus\""));
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
fn malformed_or_package_less_local_manifests_are_not_trusted() {
    let root = tempfile::tempdir().expect("isolated manifest directory");
    let candidate = root.path().join("candidate");
    fs::create_dir(&candidate).expect("candidate directory");

    fs::write(candidate.join("Cargo.toml"), "[package\nname = broken").expect("malformed manifest");
    assert!(!is_matching_local_package(
        &candidate,
        "candidate",
        "12.0.0-rc.1"
    ));

    fs::write(candidate.join("Cargo.toml"), "[workspace]\nmembers = []\n")
        .expect("package-less manifest");
    assert!(!is_matching_local_package(
        &candidate,
        "candidate",
        "12.0.0-rc.1"
    ));
}

#[test]
fn source_checkout_fallback_matches_the_current_release_channel() {
    let version = env!("CARGO_PKG_VERSION");
    let source =
        dependency_source(Path::new("/tmp"), "rullst", version).expect("current dependency source");
    if version.contains('-') {
        assert!(source.starts_with("path = "), "unexpected source: {source}");
    } else {
        assert_eq!(source, format!("version = \"{version}\""));
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
fn relational_hot_reload_does_not_add_a_duplicate_database_bootstrap_dependency() {
    let manifest = build_cargo_toml(
        "hot-sqlite-app",
        true,
        true,
        "Sqlite",
        &[],
        false,
        false,
        BLANK_BLUEPRINT_ID,
        "Zero-Bundle HTMX",
        &isolated_root(),
    )
    .expect("hot SQLite manifest");

    assert!(!manifest.contains("dotenvy = \"0.15\""));
    assert!(manifest.contains("edition = \"2024\""));
}

#[test]
fn generated_projects_use_the_frameworks_rust_edition() {
    for hot_reload in [false, true] {
        let manifest = build_cargo_toml(
            "edition-app",
            hot_reload,
            false,
            "Sqlite",
            &[],
            false,
            false,
            BLANK_BLUEPRINT_ID,
            "Zero-Bundle HTMX",
            &isolated_root(),
        )
        .expect("generated manifest");

        assert!(manifest.contains("edition = \"2024\""));
        assert!(manifest.contains("rust-version = \"1.96.0\""));
        assert!(!manifest.contains("edition = \"2021\""));
    }
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
        // The direct dependency's `drivers-all` default compiled all three
        // drivers, including bundled SQLite, into every strict profile.
        assert_eq!(
            parsed["dependencies"]["rullst-orm"]["default-features"].as_bool(),
            Some(false),
            "{provider}: the direct rullst-orm dependency must not re-enable every driver"
        );
    }
}

#[test]
fn orm_without_a_strict_profile_keeps_its_any_drivers() {
    // Turso-primary and database-free Redis profiles use `AnyPool`, which
    // needs the drivers that the default `drivers-all` feature installs.
    for (db_needed, provider, wants_redis) in [(true, "Turso", false), (false, "Sqlite", true)] {
        let manifest = build_cargo_toml(
            "any-app",
            false,
            db_needed,
            provider,
            &[],
            false,
            wants_redis,
            BLANK_BLUEPRINT_ID,
            "Zero-Bundle HTMX",
            &isolated_root(),
        )
        .expect("non-strict manifest");
        let parsed: toml::Value = toml::from_str(&manifest).expect("valid Cargo manifest");
        assert!(
            parsed["dependencies"]["rullst-orm"]
                .get("default-features")
                .is_none(),
            "{provider}: {manifest}"
        );
    }
}

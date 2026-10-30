use super::*;
use crate::blueprints::SAAS_BLUEPRINT_ID;

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
    let blank = build_cargo_toml(
        "blank",
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
    .expect("Blank manifest");
    assert!(saas.contains("\"nexus\", \"auth\", \"mail\", \"capital\""));
    assert!(saas.contains("rullst-auth = "));
    for feature in ["\"nexus\"", "\"auth\"", "\"mail\"", "\"capital\""] {
        assert!(!blank.contains(feature), "{feature}");
    }
    assert!(!blank.contains("rullst-auth = "));
}

#[test]
fn starters_declare_only_the_framework_crates_their_code_uses() {
    let direct = |blueprint| {
        let manifest = build_cargo_toml(
            "direct",
            false,
            true,
            "Sqlite",
            &[],
            false,
            false,
            blueprint,
            "Zero-Bundle HTMX",
            &isolated_root(),
        )
        .expect("manifest");
        let parsed: toml::Value = toml::from_str(&manifest).expect("valid Cargo manifest");
        let mut names = parsed["dependencies"]
            .as_table()
            .expect("dependencies")
            .keys()
            .filter(|name| name.starts_with("rullst"))
            .cloned()
            .collect::<Vec<_>>();
        names.sort();
        names
    };
    // `make:oauth` enables the facade's `oauth-*` feature, so the SaaS
    // starter does not compile `rullst-connect` and its JWT stack up front.
    assert_eq!(
        direct(SAAS_BLUEPRINT_ID),
        [
            "rullst",
            "rullst-auth",
            "rullst-capital",
            "rullst-orm",
            "rullst-security"
        ]
    );
    assert_eq!(
        direct(BLANK_BLUEPRINT_ID),
        ["rullst", "rullst-orm", "rullst-security"]
    );
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
            format!("{crate_name} = {{ version = \"=12.0.0-rc.7\" }}\n")
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
        "version = \"=12.0.0-rc.7\""
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
    // The starter reads its Turso settings with `rullst::config::project_setting`.
    assert!(!manifest.contains("dotenvy"));
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
        // SQLx TLS uses aws-lc-rs, the framework's rustls provider.
        let sqlx = parsed["dependencies"]["sqlx"]["features"]
            .as_array()
            .expect("generated SQLx feature array")
            .iter()
            .filter_map(toml::Value::as_str)
            .filter(|feature| feature.starts_with("tls-"))
            .collect::<Vec<_>>();
        assert_eq!(sqlx, ["tls-rustls-aws-lc-rs"], "{provider}");
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

#[test]
fn release_profile_is_optimized_and_keeps_unwinding_panics() {
    for hot_reload in [false, true] {
        let manifest = build_cargo_toml(
            "release-app",
            hot_reload,
            true,
            "Sqlite",
            &[],
            false,
            false,
            BLANK_BLUEPRINT_ID,
            "Zero-Bundle HTMX",
            &isolated_root(),
        )
        .expect("manifest");
        let parsed: toml::Value = toml::from_str(&manifest).expect("valid Cargo manifest");
        let release = &parsed["profile"]["release"];
        assert_eq!(release["lto"].as_str(), Some("thin"));
        assert_eq!(release["codegen-units"].as_integer(), Some(1));
        assert_eq!(release["strip"].as_str(), Some("symbols"));
        // Aborting on panic would let one failing handler stop the server.
        assert!(release.get("panic").is_none(), "{manifest}");
        assert_eq!(
            release.as_table().map(toml::Table::len),
            Some(3),
            "{manifest}"
        );
        let profiles = parsed["profile"].as_table().expect("profile table");
        let mut names: Vec<&str> = profiles.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(names, ["dev", "release"], "{manifest}");
        assert!(parsed.get("workspace").is_some(), "{manifest}");
    }
}

#[test]
fn dev_profile_keeps_line_tables_for_the_application_only() {
    for (hot_reload, db_needed, blueprint) in [
        (false, false, BLANK_BLUEPRINT_ID),
        (false, true, BLANK_BLUEPRINT_ID),
        (true, true, BLANK_BLUEPRINT_ID),
        (false, true, SAAS_BLUEPRINT_ID),
    ] {
        let manifest = build_cargo_toml(
            "dev-app",
            hot_reload,
            db_needed,
            "Sqlite",
            &[],
            false,
            false,
            blueprint,
            "Zero-Bundle HTMX",
            &isolated_root(),
        )
        .expect("manifest");
        let parsed: toml::Value = toml::from_str(&manifest).expect("valid Cargo manifest");
        // Line tables keep file:line in the application's panics and
        // backtraces; only `debug` departs from Cargo's development defaults.
        let expected: toml::Value =
            toml::from_str("debug = \"line-tables-only\"\npackage.\"*\".debug = false\n")
                .expect("expected profile");
        assert_eq!(parsed["profile"]["dev"], expected, "{manifest}");
    }
}

#[test]
fn every_profile_declares_validator_for_validate_derives() {
    for (hot_reload, blueprint) in [
        (false, BLANK_BLUEPRINT_ID),
        (true, BLANK_BLUEPRINT_ID),
        (false, SAAS_BLUEPRINT_ID),
    ] {
        let manifest = build_cargo_toml(
            "forms",
            hot_reload,
            false,
            "Sqlite",
            &[],
            false,
            false,
            blueprint,
            "Zero-Bundle HTMX",
            &isolated_root(),
        )
        .expect("generated manifest");
        let parsed: toml::Value = toml::from_str(&manifest).expect("valid TOML");
        let validator = &parsed["dependencies"]["validator"];
        assert_eq!(validator["version"].as_str(), Some("0.21"));
        assert_eq!(
            validator["features"].as_array().map(Vec::as_slice),
            Some(&[toml::Value::String("derive".into())][..])
        );
    }
}

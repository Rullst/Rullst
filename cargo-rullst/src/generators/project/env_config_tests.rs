use super::*;

#[test]
fn nix_and_buildah_generators_emit_distinct_artifacts() {
    let root =
        std::env::temp_dir().join(format!("rullst-container-flags-{}", rand::random::<u64>()));
    fs::create_dir_all(&root).expect("temporary project");

    generate_nix_files(&root).expect("Nix files");
    assert!(root.join("flake.nix").is_file());
    assert!(root.join(".envrc").is_file());
    assert!(!root.join("build_buildah.sh").exists());

    generate_buildah_script(&root, "demo-app").expect("Buildah script");
    let script = fs::read_to_string(root.join("build_buildah.sh")).expect("Buildah source");
    assert!(script.contains("buildah bud"));
    assert!(script.contains("demo-app:latest"));

    fs::remove_dir_all(root).expect("temporary project cleanup");
}

#[test]
fn generated_environment_uses_the_canonical_rullst_name() {
    let root = std::env::temp_dir().join(format!("rullst-env-name-{}", rand::random::<u64>()));
    fs::create_dir_all(&root).expect("temporary project");

    generate_env_and_configs(
        &root,
        false,
        "Sqlite",
        &[],
        BLANK_BLUEPRINT_ID,
        "0123456789abcdef0123456789abcdef",
    )
    .expect("environment scaffold");

    for filename in [".env", ".env.example"] {
        let generated =
            fs::read_to_string(root.join(filename)).expect("generated environment file");
        assert!(generated.contains("RULLST_ENV=development"));
        assert!(!generated.contains("APP_ENV="));
        // An empty MAIL_FROM is unset; the driver stays a comment so the
        // development log fallback keeps working when the file is loaded.
        assert!(generated.contains("\nMAIL_FROM=\n"));
        assert!(generated.contains("\n# MAIL_DRIVER=resend\n"));
        assert!(
            !generated
                .lines()
                .any(|line| line.starts_with("MAIL_DRIVER="))
        );
    }

    fs::remove_dir_all(root).expect("temporary project cleanup");
}

#[test]
fn host_linker_selection_stays_out_of_the_repository() {
    let root = tempfile::tempdir().expect("temporary project");
    generate_env_and_configs(
        root.path(),
        false,
        "Sqlite",
        &[],
        BLANK_BLUEPRINT_ID,
        "0123456789abcdef0123456789abcdef",
    )
    .expect("environment scaffold");

    let config = fs::read_to_string(root.path().join(".cargo/config.toml")).expect("Cargo config");
    assert!(config.contains("Host-local"));
    let gitignore = fs::read_to_string(root.path().join(".gitignore")).expect("gitignore");
    assert!(gitignore.lines().any(|line| line == "/.cargo/config.toml"));
}

#[test]
fn selected_persistence_integrations_get_offline_safe_development_values() {
    let root = std::env::temp_dir().join(format!("rullst-polyglot-env-{}", rand::random::<u64>()));
    fs::create_dir_all(&root).expect("temporary project");

    generate_env_and_configs(
        &root,
        true,
        "MariaDB",
        &[
            PolyglotIntegration::Turso,
            PolyglotIntegration::MongoDb,
            PolyglotIntegration::DuckDb,
            PolyglotIntegration::SurrealDb,
            PolyglotIntegration::Qdrant,
        ],
        BLANK_BLUEPRINT_ID,
        "0123456789abcdef0123456789abcdef",
    )
    .expect("polyglot environment scaffold");

    let development = fs::read_to_string(root.join(".env")).expect("development env");
    let example = fs::read_to_string(root.join(".env.example")).expect("example env");
    assert!(development.contains("DATABASE_URL=mysql://"));
    assert!(development.contains("TURSO_DATABASE_URL=mock_local"));
    assert!(development.contains("MONGODB_URL=mock_local"));
    assert!(development.contains("DUCKDB_PATH=analytics.duckdb"));
    assert!(development.contains("SURREALDB_URL=mock_local"));
    assert!(development.contains("QDRANT_URL=mock_local"));
    assert!(example.contains("TURSO_DATABASE_URL=\n"));
    assert!(example.contains("MONGODB_URL=\n"));
    assert!(example.contains("SURREALDB_URL=\n"));
    assert!(example.contains("QDRANT_URL=\n"));

    fs::remove_dir_all(root).expect("temporary project cleanup");
}

#[test]
fn turso_primary_has_no_fictitious_sqlx_database_url() {
    let root = std::env::temp_dir().join(format!("rullst-turso-env-{}", rand::random::<u64>()));
    fs::create_dir_all(&root).expect("temporary project");

    generate_env_and_configs(
        &root,
        true,
        "Turso",
        &[PolyglotIntegration::Turso],
        BLANK_BLUEPRINT_ID,
        "0123456789abcdef0123456789abcdef",
    )
    .expect("Turso-primary environment scaffold");

    let development = fs::read_to_string(root.join(".env")).expect("development env");
    assert!(
        !development
            .lines()
            .any(|line| line.starts_with("DATABASE_URL="))
    );
    assert!(development.contains("TURSO_DATABASE_URL=mock_local"));
    assert!(development.contains("TURSO_OFFLINE_PATH=turso-development.db"));
    assert!(!root.join("Rullst.toml").exists());

    fs::remove_dir_all(root).expect("temporary project cleanup");
}

#[test]
fn buildah_tags_a_lowercase_image_name() {
    let root = tempfile::tempdir().expect("temporary project");
    generate_buildah_script(root.path(), "My_App").expect("Buildah script");
    let script = fs::read_to_string(root.path().join("build_buildah.sh")).expect("script");
    // `buildah bud -t My_App:latest` fails: repository names are lowercase.
    assert!(script.contains("buildah bud -f Dockerfile -t my-app:latest .\n"));
    assert!(!script.contains("My_App"));
}

#[test]
fn buildah_write_failures_are_propagated() {
    let missing_parent = std::env::temp_dir()
        .join(format!("rullst-missing-buildah-{}", rand::random::<u64>()))
        .join("project");
    assert!(generate_buildah_script(&missing_parent, "demo").is_err());
}

#[test]
// TM-DEPLOY-05: validated generator inputs cannot cross into shell syntax.
fn buildah_image_name_cannot_inject_shell_commands() {
    let root = std::env::temp_dir().join(format!("rullst-buildah-name-{}", rand::random::<u64>()));
    fs::create_dir_all(&root).expect("temporary project");
    assert!(generate_buildah_script(&root, "demo; touch compromised").is_err());
    assert!(!root.join("build_buildah.sh").exists());
    fs::remove_dir_all(root).expect("temporary project cleanup");
}

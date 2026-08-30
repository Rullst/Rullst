#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::{fs, process::Command};

#[test]
fn deterministic_cli_materializes_relational_and_specialized_persistence_choices() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let project = std::env::temp_dir().join(format!(
        "rullst-polyglot-scaffold-{}",
        rand::random::<u64>()
    ));
    let output = Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(workspace)
        .arg("new")
        .arg(&project)
        .args([
            "--default",
            "--database",
            "mariadb",
            "--turso",
            "--mongodb",
            "--duckdb",
            "--surrealdb",
            "--skip-initial-migration",
        ])
        .output()
        .expect("run deterministic project generator");
    assert!(
        output.status.success(),
        "project generation failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let manifest = fs::read_to_string(project.join("Cargo.toml")).expect("generated manifest");
    let parsed: toml::Value = toml::from_str(&manifest).expect("valid generated manifest");
    let rullst_features = parsed["dependencies"]["rullst"]["features"]
        .as_array()
        .expect("umbrella feature array")
        .iter()
        .filter_map(toml::Value::as_str)
        .collect::<Vec<_>>();
    for feature in ["orm-turso", "orm-mongodb", "orm-duckdb", "orm-surrealdb"] {
        assert!(rullst_features.contains(&feature), "missing {feature}");
    }
    assert_eq!(
        parsed["dependencies"]["sqlx"]["features"][2].as_str(),
        Some("mysql")
    );

    let environment = fs::read_to_string(project.join(".env")).expect("generated environment");
    for variable in [
        "DATABASE_URL=mysql://",
        "TURSO_DATABASE_URL=mock_local",
        "MONGODB_URL=mock_local",
        "DUCKDB_PATH=analytics.duckdb",
        "SURREALDB_URL=mock_local",
    ] {
        assert!(environment.contains(variable), "missing {variable}");
    }

    fs::remove_dir_all(&project).expect("remove generated test project");
}

#[test]
fn turso_primary_scaffold_compiles_and_runs_its_real_migration() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let project = std::env::temp_dir().join(format!(
        "rullst-turso-primary-scaffold-{}",
        rand::random::<u64>()
    ));
    let output = Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(workspace)
        .arg("new")
        .arg(&project)
        .args([
            "--default",
            "--api",
            "--database",
            "turso",
            "--skip-initial-migration",
        ])
        .output()
        .expect("run Turso-primary project generator");
    assert!(
        output.status.success(),
        "Turso-primary generation failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let manifest = fs::read_to_string(project.join("Cargo.toml")).expect("generated manifest");
    assert!(manifest.contains("\"orm-turso\""));
    assert!(!manifest.lines().any(|line| line.starts_with("sqlx = ")));
    let environment = fs::read_to_string(project.join(".env")).expect("generated environment");
    assert!(
        !environment
            .lines()
            .any(|line| line.starts_with("DATABASE_URL="))
    );

    let checked = Command::new("cargo")
        .current_dir(&project)
        .arg("check")
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .output()
        .expect("check generated Turso-primary project");
    assert!(
        checked.status.success(),
        "generated Turso-primary project did not compile:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&checked.stdout),
        String::from_utf8_lossy(&checked.stderr)
    );

    let migrated = Command::new("cargo")
        .current_dir(&project)
        .args(["run", "--quiet", "--", "db:migrate"])
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .output()
        .expect("run generated Turso migration");
    assert!(
        migrated.status.success(),
        "generated Turso migration failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&migrated.stdout),
        String::from_utf8_lossy(&migrated.stderr)
    );
    assert!(project.join("turso-development.db").exists());

    let status = Command::new("cargo")
        .current_dir(&project)
        .args(["run", "--quiet", "--", "db:status"])
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .output()
        .expect("read generated Turso migration status");
    assert!(status.status.success());
    assert!(
        String::from_utf8_lossy(&status.stdout)
            .contains("[applied] m20260829000000_create_users_table")
    );

    let rollback = Command::new("cargo")
        .current_dir(&project)
        .args(["run", "--quiet", "--", "db:rollback"])
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .output()
        .expect("roll back generated Turso migration");
    assert!(
        rollback.status.success(),
        "generated Turso rollback failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rollback.stdout),
        String::from_utf8_lossy(&rollback.stderr)
    );
    assert!(
        String::from_utf8_lossy(&rollback.stdout)
            .contains("Rolled back Turso migration m20260829000000_create_users_table")
    );

    fs::remove_dir_all(&project).expect("remove generated Turso-primary test project");
}

#[test]
fn turso_primary_make_model_and_migration_remain_on_the_libsql_backend() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let project =
        std::env::temp_dir().join(format!("rullst-turso-generators-{}", rand::random::<u64>()));
    let cli = env!("CARGO_BIN_EXE_rullst");
    let generated = Command::new(cli)
        .current_dir(workspace)
        .arg("new")
        .arg(&project)
        .args([
            "--default",
            "--api",
            "--database",
            "turso",
            "--skip-initial-migration",
        ])
        .output()
        .expect("generate Turso-primary project");
    assert!(generated.status.success());

    let model = Command::new(cli)
        .current_dir(&project)
        .args(["make:model", "Widget", "--migration"])
        .output()
        .expect("generate typed Turso model and migration");
    assert!(
        model.status.success(),
        "make:model failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&model.stdout),
        String::from_utf8_lossy(&model.stderr)
    );
    let migration = Command::new(cli)
        .current_dir(&project)
        .args(["make:migration", "create_audit_entries"])
        .output()
        .expect("generate Turso migration");
    assert!(migration.status.success());

    let model_source =
        fs::read_to_string(project.join("src/models/widget.rs")).expect("generated model");
    assert!(model_source.contains("backend = \"turso\""));
    assert!(!model_source.contains("FromRow"));
    let migrations_mod =
        fs::read_to_string(project.join("src/migrations/mod.rs")).expect("migration registry");
    assert!(migrations_mod.contains("Vec<rullst_orm::polyglot::TursoMigration>"));
    assert!(migrations_mod.contains("::migration()?"));

    let checked = Command::new("cargo")
        .current_dir(&project)
        .arg("check")
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .output()
        .expect("check Turso project after generators");
    assert!(
        checked.status.success(),
        "generated Turso additions did not compile:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&checked.stdout),
        String::from_utf8_lossy(&checked.stderr)
    );

    let migrated = Command::new("cargo")
        .current_dir(&project)
        .args(["run", "--quiet", "--", "db:migrate"])
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .output()
        .expect("run all generated Turso migrations");
    assert!(
        migrated.status.success(),
        "generated Turso migrations failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&migrated.stdout),
        String::from_utf8_lossy(&migrated.stderr)
    );
    assert!(String::from_utf8_lossy(&migrated.stdout).contains("Applied 3 Turso migration(s)"));

    fs::remove_dir_all(&project).expect("remove generated Turso project");
}

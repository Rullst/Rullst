//! Rules added in 13.0.0-rc.1 for changes of 13.0.0-alpha.4 that no rule
//! located: SQLx's ring TLS with native roots, the deployment `Dockerfile`'s
//! SQLite `DATABASE_URL`, the `Caddyfile`'s health checks, and scripts that
//! run `make:migration:auto` or a removed `--blueprint`.

use super::super::files::{passes_removed_blueprint, runs_migration_auto};
use super::alpha4_orm_nexus::manifest;
use super::files::Project;
use super::*;

const NATIVE: &str = "V13-SQLX-TLS-NATIVE-ROOTS";
const PACKAGE: &str = "[package]\nname = \"app\"\n";

#[test]
fn sqlx_dependencies_with_ring_native_roots_are_reviewed() {
    for (text, line) in [
        (
            "[dependencies]\nsqlx = { version = \"0.9\", features = [\"runtime-tokio\", \"tls-rustls-ring-native-roots\", \"postgres\"] }\n",
            2,
        ),
        (
            "[package]\nname = \"app\"\n\n[dependencies.sqlx]\nversion = \"0.9\"\nfeatures = [\"tls-rustls-ring-native-roots\"]\n",
            4,
        ),
        (
            "[workspace]\nmembers = [\"app\"]\n\n[workspace.dependencies]\nsqlx = { version = \"0.9\", features = [\"tls-rustls-ring-native-roots\"] }\n",
            5,
        ),
        (
            "[dependencies]\ndatabase = { package = \"sqlx\", version = \"0.9\", features = [\"tls-rustls-ring-native-roots\"] }\n",
            2,
        ),
        // Enabled from `[features]`, also through an optional dependency.
        (
            "[dependencies]\nsqlx = { version = \"0.9\", optional = true }\n\n[features]\n# \"sqlx/tls-rustls-ring-native-roots\"\ndb = [\"dep:sqlx\", \"sqlx?/tls-rustls-ring-native-roots\"]\n",
            6,
        ),
        // Once per manifest, at the first declaration.
        (
            "[dependencies]\nsqlx = { version = \"0.9\", features = [\"tls-rustls-ring-native-roots\"] }\n\n[dev-dependencies.sqlx]\nversion = \"0.9\"\nfeatures = [\"tls-rustls-ring-native-roots\"]\n",
            2,
        ),
    ] {
        assert_eq!(manifest(text).0, [(NATIVE, line)], "{text}");
    }
    for text in [
        "[dependencies]\nsqlx = { version = \"0.9\", features = [\"tls-rustls-aws-lc-rs\"] }\n",
        "[dependencies]\nsqlx = { version = \"0.9\", features = [\"tls-native-tls\"] }\n",
        "[dependencies]\nother = { version = \"1\", features = [\"tls-rustls-ring-native-roots\"] }\n\n[features]\ntls = [\"other/tls-rustls-ring-native-roots\", \"tls-rustls-ring-native-roots\"]\n",
    ] {
        assert_eq!(manifest(text).0, [], "{text}");
    }
}

/// Each SQLx TLS rule is reported on its own, at its own first line.
#[test]
fn ring_with_webpki_and_with_native_roots_are_two_findings() {
    let text = "[dependencies]\nsqlx = { version = \"0.9\", features = [\"tls-rustls-ring-native-roots\"] }\n\n[features]\ntls = [\"sqlx/tls-rustls\"]\n";
    assert_eq!(
        manifest(text).0,
        [("V13-SQLX-TLS-RING", 5), (NATIVE, 2)],
        "{text}"
    );
}

#[test]
fn the_rc1_rules_review_the_rows_of_their_alpha4_changes() {
    let rc1 = Version::parse("13.0.0-rc.1").unwrap();
    let alpha4 = Version::parse("13.0.0-alpha.4").unwrap();
    for (rule, row) in [
        (&catalog::SQLX_TLS_NATIVE_ROOTS, "TLS crypto provider"),
        (
            &catalog::DEPLOY_SQLITE_URL,
            "SQLite database in the deployed image",
        ),
        (&catalog::DEPLOY_CADDY_HEALTH, "CLI deploy targets removed"),
        (
            &catalog::MIGRATION_AUTO_STATUS,
            "CLI `make:migration:auto` without a database to compare",
        ),
        (
            &catalog::BLUEPRINTS_REMOVED,
            "CLI starters moved to the examples",
        ),
    ] {
        assert_eq!(rule.kind, FindingKind::Review, "{}", rule.code);
        assert_eq!(rule.row, row, "{}", rule.code);
        assert!(rule.introduced_after(&alpha4), "{}", rule.code);
        assert!(!rule.introduced_after(&rc1), "{}", rule.code);
    }
    let native = &catalog::SQLX_TLS_NATIVE_ROOTS;
    assert!(native.guidance.contains("`tls-rustls-aws-lc-rs`"));
    assert!(native.guidance.contains("`sslrootcert`"));
}

const SQLITE_CONFIG: &str = "[database]\nurl = \"sqlite://db.sqlite\"\n";
const OLD_DOCKERFILE: &str = "FROM rust:1.96 AS builder\nRUN cargo build --release --locked\n\n  from debian:bookworm-slim\nCOPY --from=builder /app/target/release/app /app/app\n";

#[test]
fn a_sqlite_dockerfile_without_database_url_is_reviewed_at_its_runtime_stage() {
    let project = Project::new();
    project
        .write("Rullst.toml", SQLITE_CONFIG)
        .write("Dockerfile", OLD_DOCKERFILE);
    assert_eq!(
        project.findings(PACKAGE),
        [("Dockerfile".to_string(), "V13-DEPLOY-SQLITE-URL", 4)]
    );
    // Without a stage line the finding points at the file.
    project.write("Dockerfile", "RUN cargo build --release --locked\n");
    assert_eq!(
        project.findings(PACKAGE),
        [("Dockerfile".to_string(), "V13-DEPLOY-SQLITE-URL", 1)]
    );
    // `.env.example` names the database when `Rullst.toml` has no URL.
    let example = Project::new();
    example
        .write(
            ".env.example",
            "APP_KEY=\nDATABASE_URL=sqlite://db.sqlite?mode=rwc\n",
        )
        .write("Dockerfile", OLD_DOCKERFILE);
    assert_eq!(
        example.findings(PACKAGE),
        [("Dockerfile".to_string(), "V13-DEPLOY-SQLITE-URL", 4)]
    );
}

#[test]
fn dockerfiles_with_database_url_or_without_sqlite_are_not_reviewed() {
    let with_url =
        format!("{OLD_DOCKERFILE}ENV DATABASE_URL=sqlite:///app/data/rullst.db?mode=rwc\n");
    let postgres = "[database]\nurl = \"postgres://app@db/app\"\n";
    for (config, dockerfile) in [
        (SQLITE_CONFIG, with_url.as_str()),
        (postgres, OLD_DOCKERFILE),
        ("[security]\n", OLD_DOCKERFILE),
    ] {
        let project = Project::new();
        project
            .write("Rullst.toml", config)
            .write("Dockerfile", dockerfile);
        assert_eq!(project.findings(PACKAGE), [], "{config}{dockerfile}");
    }
    let without_dockerfile = Project::new();
    without_dockerfile.write("Rullst.toml", SQLITE_CONFIG);
    assert_eq!(without_dockerfile.findings(PACKAGE), []);
}

#[test]
fn a_caddyfile_without_health_checks_is_reviewed_at_its_proxy() {
    let earlier = "{$DOMAIN:localhost} {\n    reverse_proxy app:3000\n}\n";
    let only_health =
        "{$DOMAIN:localhost} {\n\treverse_proxy app:3000 {\n\t\thealth_uri /health\n\t}\n}\n";
    let only_hold =
        "{$DOMAIN:localhost} {\n\treverse_proxy app:3000 {\n\t\tlb_try_duration 15s\n\t}\n}\n";
    let commented = "{$DOMAIN:localhost} {\n\treverse_proxy app:3000 {\n\t\t# health_uri /health\n\t\t# lb_try_duration 15s\n\t}\n}\n";
    for caddyfile in [earlier, only_health, only_hold, commented] {
        let project = Project::new();
        project.write("Caddyfile", caddyfile);
        assert_eq!(
            project.findings(PACKAGE),
            [("Caddyfile".to_string(), "V13-DEPLOY-CADDY-HEALTH", 2)],
            "{caddyfile}"
        );
    }
    let other_proxy = "example.com {\n    reverse_proxy web:8080\n}\n";
    for caddyfile in [crate::blueprints::deploy::CADDYFILE, other_proxy] {
        let project = Project::new();
        project.write("Caddyfile", caddyfile);
        assert_eq!(project.findings(PACKAGE), [], "{caddyfile}");
    }
}

#[test]
fn scripts_running_migration_auto_or_a_removed_blueprint_are_reviewed() {
    let project = Project::new();
    project
        .write(
            ".github/workflows/ci.yml",
            "steps:\n  - run: cargo rullst db:migrate\n  - run: cargo rullst make:migration:auto\n",
        )
        .write(
            "Makefile",
            "demo:\n\tcargo rullst new demo \\\n\t\t--blueprint lms --default\n",
        );
    let mut found = project.findings(PACKAGE);
    found.sort();
    assert_eq!(
        found,
        [
            (
                ".github/workflows/ci.yml".to_string(),
                "V13-CLI-MIGRATION-AUTO-STATUS",
                3
            ),
            ("Makefile".to_string(), "V13-CLI-BLUEPRINTS-REMOVED", 3),
        ]
    );
    assert!(runs_migration_auto(
        "cargo rullst make:migration:auto --dry-run"
    ));
    assert!(!runs_migration_auto(
        "cargo rullst make:migration add_users"
    ));
}

#[test]
fn only_the_blueprints_moved_to_the_examples_are_removed() {
    for removed in [
        "cargo rullst new app --blueprint lms",
        "cargo rullst new app --blueprint=erp",
        "cargo rullst new app --blueprint \"portfolio\"",
        "cargo rullst new app --blueprint  'blog' --default",
    ] {
        assert!(passes_removed_blueprint(removed), "{removed}");
    }
    for kept in [
        "cargo rullst new app --blueprint api",
        "cargo rullst new app --blueprint=saas",
        "cargo rullst new blog",
        "cargo rullst new lms --default",
        "--blueprint",
    ] {
        assert!(!passes_removed_blueprint(kept), "{kept}");
    }
}

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::project::{Database, database};
use super::*;
use std::fs;
use std::path::PathBuf;

const TIMESTAMP: &str = "20261022120000";
const MIGRATION: &str = "m20261022120000_create_oauth_identities_table";

/// A generated starter with `database` in a temporary directory.
fn starter(blueprint: usize, hot: bool, database: &str) -> (tempfile::TempDir, PathBuf) {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("app");
    fs::create_dir_all(&root).unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let manifest = crate::generators::project::cargo_toml::build_cargo_toml(
        "oauth-app",
        hot,
        true,
        database,
        &[],
        false,
        false,
        blueprint,
        "Zero-Bundle HTMX",
        workspace,
    )
    .unwrap();
    fs::write(root.join("Cargo.toml"), manifest).unwrap();
    fs::write(root.join(".env.example"), "APP_KEY=\n").unwrap();
    crate::blueprints::apply(
        blueprint,
        &root,
        "oauth-app",
        "oauth_app",
        false,
        hot,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .unwrap();
    (temporary, root)
}

fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative)).unwrap()
}

fn installed(root: &Path) -> Vec<String> {
    installed_provider(root, "google")
}

fn installed_provider(root: &Path, name: &str) -> Vec<String> {
    let plan = plan(root, name, TIMESTAMP).unwrap();
    writes::apply(&plan.edits).unwrap();
    plan.notes
}

#[test]
fn the_saas_starter_gets_routes_link_store_identities_and_features() {
    let (_temporary, root) = starter(crate::blueprints::SAAS_BLUEPRINT_ID, false, "Sqlite");
    let notes = installed(&root);
    assert!(
        notes
            .iter()
            .all(|note| !note.starts_with("Mount the routes"))
    );
    assert!(notes.iter().all(|note| !note.starts_with("Add {crate::")));
    assert!(
        notes
            .iter()
            .any(|note| note.contains("--features oauth-mock"))
    );

    let router = read(&root, "src/main.rs");
    let login = router
        .find("controllers::auth_controller::login_submit")
        .unwrap();
    let start = router
        .find(".route(\"/auth/google\", rullst::routing::get(controllers::oauth::google::start)")
        .unwrap();
    let callback = router
        .find(".route(\"/auth/google/callback\", rullst::routing::get(controllers::oauth::google::callback)")
        .unwrap();
    assert!(login < start && start < callback);
    // Both routes carry the password login's per-client budget.
    let budget =
        ".layer(rullst::server::from_fn(controllers::auth_controller::credential_rate_limit)))";
    assert!(router[start..callback].contains(budget));
    assert!(router[callback..].starts_with(&format!(
        ".route(\"/auth/google/callback\", rullst::routing::get(controllers::oauth::google::callback)\n        {budget}"
    )));
    let initialize = router
        .find("controllers::oauth::google::initialize()?;")
        .unwrap();
    assert!(router.find("fn router()").unwrap() < initialize && initialize < login);
    assert!(router.find("csrf_middleware").unwrap() > callback);

    let page = read(&root, "src/pages/auth.rs");
    let link = page
        .find("{crate::controllers::oauth::google::login_link()}")
        .unwrap();
    assert!(page.find("action=\"/login\"").unwrap() < link);
    assert!(page.find("Forgot your password?").unwrap() > link);

    let controller = read(&root, "src/controllers/oauth/mod.rs");
    assert!(controller.contains("pub mod google;"));
    assert!(controller.contains("SqlChallengeStore<rullst::db::sqlx::Sqlite>"));
    assert!(controller.contains(
        "pub const EXISTING_ACCOUNT_POLICY: ExistingAccountPolicy = ExistingAccountPolicy::Refuse;"
    ));
    assert!(controller.contains("rullst_auth::make_login_cookie_with_epoch("));
    assert!(
        controller.contains("if profile.email_verified != Some(true) || !valid_email(&email) {")
    );
    let google = read(&root, "src/controllers/oauth/google.rs");
    assert!(google.contains("LoginOptions::oidc(PROVIDER)"));
    for source in [
        &controller,
        &google,
        &read(&root, "src/models/oauth_identity.rs"),
    ] {
        assert!(!source.contains("__"), "unreplaced placeholder");
        for forbidden in [".unwrap(", ".expect(", "panic!("] {
            assert!(!source.contains(forbidden), "{forbidden}");
        }
    }
    assert!(read(&root, "src/controllers/mod.rs").contains("pub mod oauth;\n"));
    assert!(read(&root, "src/models/mod.rs").contains("pub mod oauth_identity;\n"));

    let migration = read(&root, &format!("src/migrations/{MIGRATION}.rs"));
    assert!(migration.contains(&format!("\"{MIGRATION}\"")));
    assert!(migration.contains("table.unique(&[\"provider\", \"subject\"]);"));
    assert!(migration.contains(".foreign_id(\"user_id\", \"users\")"));
    let migrations = read(&root, "src/migrations/mod.rs");
    assert!(migrations.contains(&format!("pub mod {MIGRATION};\n")));
    let users = migrations
        .find("Box::new(m20260601000000_create_users_table::MigrationImpl)")
        .unwrap();
    assert!(
        migrations
            .find(&format!("Box::new({MIGRATION}::MigrationImpl)"))
            .unwrap()
            > users
    );

    let manifest: toml::Table = toml::from_str(&read(&root, "Cargo.toml")).unwrap();
    let features = manifest["dependencies"]["rullst"]["features"]
        .as_array()
        .unwrap();
    for feature in ["oauth", "oauth-sqlite", "strict-sqlite"] {
        assert!(
            features.iter().any(|value| value.as_str() == Some(feature)),
            "{feature}"
        );
    }
    assert_eq!(
        manifest["features"]["oauth-mock"].as_array().unwrap()[0].as_str(),
        Some("rullst/oauth-mock")
    );
    let env = read(&root, ".env.example");
    assert!(env.starts_with("APP_KEY=\n"));
    assert!(env.contains("\nGOOGLE_CLIENT_ID=\nGOOGLE_CLIENT_SECRET=\n"));
    assert!(env.contains("\nGOOGLE_REDIRECT_URL=\n"));
}

#[test]
fn hot_reload_lms_and_postgres_starters_are_recognized() {
    for (blueprint, hot, database, sqlx) in [
        (
            crate::blueprints::SAAS_BLUEPRINT_ID,
            true,
            "Postgres",
            "Postgres",
        ),
        (crate::blueprints::LMS_BLUEPRINT_ID, true, "MySQL", "MySql"),
        (
            crate::blueprints::LMS_BLUEPRINT_ID,
            false,
            "Sqlite",
            "Sqlite",
        ),
    ] {
        let (_temporary, root) = starter(blueprint, hot, database);
        let notes = installed(&root);
        assert!(
            notes
                .iter()
                .all(|note| !note.starts_with("Mount the routes")),
            "{database}"
        );
        let router = if hot { "src/lib.rs" } else { "src/main.rs" };
        assert!(read(&root, router).contains("controllers::oauth::google::initialize()?;"));
        assert!(read(&root, router).contains("\"/auth/google/callback\""));
        assert!(read(&root, "src/pages/auth.rs").contains("login_link()}"));
        assert!(
            read(&root, "src/controllers/oauth/mod.rs")
                .contains(&format!("SqlChallengeStore<rullst::db::sqlx::{sqlx}>"))
        );
    }
}

#[test]
fn a_second_run_and_existing_outputs_change_nothing() {
    let (_temporary, root) = starter(crate::blueprints::SAAS_BLUEPRINT_ID, false, "Sqlite");
    installed(&root);
    let manifest = read(&root, "Cargo.toml");
    let error = plan(&root, "google", TIMESTAMP).err().unwrap();
    assert!(error.to_string().contains("already"), "{error}");
    assert_eq!(read(&root, "Cargo.toml"), manifest);

    let (_temporary, root) = starter(crate::blueprints::SAAS_BLUEPRINT_ID, false, "Sqlite");
    fs::create_dir_all(root.join("src/controllers/oauth")).unwrap();
    fs::write(root.join("src/controllers/oauth/google.rs"), "// mine\n").unwrap();
    // The shared module is missing, so this is a first install: it refuses
    // to replace the application's own google.rs.
    assert!(plan(&root, "google", TIMESTAMP).is_err());
    assert_eq!(read(&root, "src/controllers/oauth/google.rs"), "// mine\n");
}

#[test]
fn projects_without_the_account_model_are_refused() {
    let (_temporary, root) = starter(crate::blueprints::BLANK_BLUEPRINT_ID, false, "Sqlite");
    let error = plan(&root, "google", TIMESTAMP).err().unwrap().to_string();
    assert!(error.contains("account model"), "{error}");

    let (_temporary, root) = starter(crate::blueprints::SAAS_BLUEPRINT_ID, false, "Sqlite");
    let user =
        read(&root, "src/models/user.rs").replace("pub session_epoch: i64,", "pub epoch: i64,");
    fs::write(root.join("src/models/user.rs"), user).unwrap();
    let error = plan(&root, "google", TIMESTAMP).err().unwrap().to_string();
    assert!(error.contains("account model"), "{error}");
    assert!(!root.join("src/controllers/oauth").exists());

    let (_temporary, root) = starter(crate::blueprints::SAAS_BLUEPRINT_ID, false, "Sqlite");
    let controller = read(&root, "src/controllers/auth_controller.rs")
        .replace("pub(crate) fn get_csrf_token", "fn get_csrf_token");
    fs::write(root.join("src/controllers/auth_controller.rs"), controller).unwrap();
    assert!(plan(&root, "google", TIMESTAMP).is_err());
}

#[test]
fn unknown_providers_and_bad_timestamps_are_refused() {
    let (_temporary, root) = starter(crate::blueprints::SAAS_BLUEPRINT_ID, false, "Sqlite");
    let error = plan(&root, "facebook", TIMESTAMP)
        .err()
        .unwrap()
        .to_string();
    assert_eq!(
        error,
        "make:oauth supports only: google, github, microsoft, apple"
    );
    for timestamp in ["2026102212000", "2026102212000a", "202610221200001"] {
        assert!(plan(&root, "google", timestamp).is_err(), "{timestamp}");
    }
    assert!(!root.join("src/controllers/oauth").exists());
    assert_eq!(
        command()
            .try_get_matches_from(["make:oauth", "facebook"])
            .unwrap_err()
            .kind(),
        clap::error::ErrorKind::InvalidValue
    );
}

#[test]
fn every_provider_is_a_command_value() {
    for name in ["google", "github", "microsoft", "apple"] {
        let matches = command()
            .try_get_matches_from(["make:oauth", name])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("provider").map(String::as_str),
            Some(name)
        );
    }
}

#[test]
fn the_database_comes_from_one_strict_profile() {
    let manifest = |features: &str| {
        format!(
            "[package]\nname = \"a\"\n\n[dependencies]\nrullst = {{ version = \"13\", features = [{features}] }}\n"
        )
    };
    assert_eq!(
        database(&manifest("\"orm\", \"strict-sqlite\"")).unwrap(),
        Database::Sqlite
    );
    assert_eq!(
        database(&manifest("\"strict-postgres\"")).unwrap(),
        Database::Postgres
    );
    assert_eq!(
        database(&manifest("\"strict-mysql\"")).unwrap(),
        Database::MySql
    );
    for refused in [
        "\"orm\"",
        "\"strict-sqlite\", \"strict-postgres\"",
        "\"orm-turso\", \"strict-sqlite\"",
    ] {
        assert!(database(&manifest(refused)).is_err(), "{refused}");
    }
    let orm_only = "[dependencies]\nrullst = \"13\"\nrullst-orm = { version = \"13\", features = [\"strict-mysql\"] }\n";
    assert_eq!(database(orm_only).unwrap(), Database::MySql);
    assert!(database("[dependencies]\nserde = \"1\"\n").is_err());
    assert_eq!(Database::Postgres.feature(), "oauth-postgres");
    assert_eq!(Database::MySql.sqlx_type(), "rullst::db::sqlx::MySql");
}

#[test]
fn an_unrecognized_router_and_page_become_instructions() {
    let (_temporary, root) = starter(crate::blueprints::SAAS_BLUEPRINT_ID, false, "Sqlite");
    let router = read(&root, "src/main.rs").replace(
        "rullst::routing::post(controllers::auth_controller::login_submit)",
        "rullst::routing::post(controllers::auth_controller::my_login)",
    );
    fs::write(root.join("src/main.rs"), &router).unwrap();
    let page = read(&root, "src/pages/auth.rs").replace("action=\"/login\"", "action=\"/sign-in\"");
    fs::write(root.join("src/pages/auth.rs"), &page).unwrap();
    let notes = installed(&root);
    assert_eq!(read(&root, "src/main.rs"), router);
    assert_eq!(read(&root, "src/pages/auth.rs"), page);
    assert!(
        notes
            .iter()
            .any(|note| note.starts_with("Mount the routes yourself"))
    );
    assert!(
        notes.iter().any(|note| note
            == "Add {crate::controllers::oauth::google::login_link()} to your login page.")
    );
}

#[path = "providers_tests.rs"]
mod providers_tests;

//! `make:oauth` for an application with its own sign-in, through its
//! authentication contract, and the starter flow after the template split.
#![allow(clippy::unwrap_used)]

use super::*;
use std::fs;
use std::path::PathBuf;

const TIMESTAMP: &str = "20261110120000";

fn own_sign_in() -> (tempfile::TempDir, PathBuf) {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap().join("app");
    fs::create_dir_all(&root).unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let manifest = crate::generators::project::cargo_toml::build_cargo_toml(
        "contract-app",
        false,
        true,
        "Sqlite",
        &[],
        false,
        false,
        crate::blueprints::SAAS_BLUEPRINT_ID,
        "Zero-Bundle HTMX",
        workspace,
    )
    .unwrap();
    fs::write(root.join("Cargo.toml"), manifest).unwrap();
    crate::blueprints::apply(
        crate::blueprints::SAAS_BLUEPRINT_ID,
        &root,
        "contract-app",
        "contract_app",
        false,
        false,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .unwrap();
    crate::generators::auth_contract::own_sign_in(&root);
    (temporary, root)
}

fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative)).unwrap()
}

#[test]
fn an_application_with_its_own_sign_in_gets_the_contract_flow() {
    let (_temporary, root) = own_sign_in();
    let router = read(&root, "src/main.rs");
    let page = read(&root, "src/pages/auth.rs");
    let planned = plan(&root, "google", TIMESTAMP).unwrap();
    writes::apply(&planned.edits).unwrap();

    let flow = read(&root, "src/controllers/oauth/mod.rs");
    for expected in [
        "use crate::auth_contract::{self, ContractAccount};",
        "auth_contract::create_account(&mut transaction, email, &name)",
        "pub async fn credential_rate_limit(",
        "mod auth {",
        "let session = match session_cookie(&user).await {",
    ] {
        assert!(flow.contains(expected), "{expected}");
    }
    for absent in [
        "crate::models::user",
        "LinkAndResetPassword",
        "auth_controller",
        "__RULLST",
    ] {
        assert!(!flow.contains(absent), "{absent}");
    }
    let identity = read(&root, "src/models/oauth_identity.rs");
    assert!(identity.contains("pub async fn link_with_tx("));
    assert!(!identity.contains("remove_password_and_sessions_with_tx"));
    let migration = read(
        &root,
        "src/migrations/m20261110120000_create_oauth_identities_table.rs",
    );
    assert!(migration.contains("table.integer(\"user_id\").not_null();"));
    assert!(migration.contains("use rullst::db::schema::{Migration, Schema};"));
    assert!(!migration.contains("foreign_id"));
    // The application's router and sign-in page stay its own.
    assert_eq!(read(&root, "src/main.rs"), router);
    assert_eq!(read(&root, "src/pages/auth.rs"), page);
    let notes = planned.notes.join("\n");
    assert!(notes.contains("Mount the routes yourself"), "{notes}");
    assert!(
        notes.contains("each with controllers::oauth::credential_rate_limit."),
        "{notes}"
    );
    assert!(
        notes.contains("Add {crate::controllers::oauth::google::login_link()} to your login page.")
    );
    assert!(notes.contains("auth_contract::create_account"));
    // A second provider only adds its module.
    let planned = plan(&root, "github", TIMESTAMP).unwrap();
    writes::apply(&planned.edits).unwrap();
    assert!(read(&root, "src/controllers/oauth/mod.rs").contains("pub mod github;"));
}

#[test]
fn the_contract_must_offer_what_oauth_calls() {
    let (_temporary, root) = own_sign_in();
    let contract = read(&root, "src/auth_contract.rs").replace("fn create_account(", "fn create(");
    fs::write(root.join("src/auth_contract.rs"), contract).unwrap();
    let error = plan(&root, "google", TIMESTAMP).err().unwrap().to_string();
    assert!(
        error.contains("lacks `pub async fn create_account` ("),
        "{error}"
    );
    fs::remove_file(root.join("src/auth_contract.rs")).unwrap();
    let error = plan(&root, "google", TIMESTAMP).err().unwrap().to_string();
    assert!(
        error.ends_with("this project has no src/models/user.rs"),
        "{error}"
    );
    assert!(error.contains("src/auth_contract.rs"), "{error}");
    assert!(!root.join("src/controllers/oauth").exists());
}

#[test]
fn the_starter_flow_keeps_its_accounts_after_the_split() {
    for second_step in [false, true] {
        let flow = render_flow(
            "google",
            "rullst::db::sqlx::Sqlite",
            Accounts::Starter,
            second_step,
        );
        syn::parse_file(&flow).unwrap();
        for expected in [
            "use crate::models::user::User;",
            "use crate::pages::auth;",
            "LinkAndResetPassword,",
            "pub(crate) async fn session_cookie(user: &User)",
            "oauth_identity::remove_password_and_sessions_with_tx(&mut transaction, user_id)",
            "pub mod google;",
            "SqlChallengeStore<rullst::db::sqlx::Sqlite>",
        ] {
            assert!(flow.contains(expected), "{expected}");
        }
        assert!(!flow.contains("__"), "unresolved placeholder");
        assert_eq!(flow.contains("mfa::second_step"), second_step);
        let contract = render_flow(
            "google",
            "rullst::db::sqlx::Sqlite",
            Accounts::Contract,
            second_step,
        );
        syn::parse_file(&contract).unwrap();
        assert!(!contract.contains("__"), "unresolved placeholder");
        assert_eq!(contract.contains("mfa::second_step"), second_step);
    }
    let (identity, migration) = render_identity(Accounts::Starter, "m1");
    assert!(identity.contains("pub async fn remove_password_and_sessions_with_tx("));
    assert!(migration.contains(".foreign_id(\"user_id\", \"users\")"));
    assert!(migration.contains("use rullst::db::schema::{ForeignKeyAction, Migration, Schema};"));
    syn::parse_file(&identity).unwrap();
    syn::parse_file(&migration).unwrap();
    let (identity, migration) = render_identity(Accounts::Contract, "m1");
    syn::parse_file(&identity).unwrap();
    syn::parse_file(&migration).unwrap();
    assert!(!migration.contains("__"));
}

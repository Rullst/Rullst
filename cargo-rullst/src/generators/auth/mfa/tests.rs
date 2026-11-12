//! `make:mfa` plans: the pages, the second sign-in step and the reset
//! command for the starter's accounts and for the authentication contract.
use super::*;
use crate::blueprints::{self, SAAS_BLUEPRINT_ID};
use std::fs;
use std::path::PathBuf;

const TIMESTAMP: &str = "20261110120000";
const MANIFEST: &str = "[package]\nname = \"acme\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = { version = \"13\", default-features = false, features = [\"orm\", \"strict-sqlite\"] }\n";
const CONTRACT: &str = include_str!("../../auth_contract/reference.rs.template");

fn saas(temporary: &Path, hot: bool) -> PathBuf {
    let root = writes::symlinked_project(temporary);
    fs::write(root.join("Cargo.toml"), MANIFEST).unwrap();
    blueprints::apply(
        SAAS_BLUEPRINT_ID,
        &root,
        "acme",
        "acme",
        false,
        hot,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .unwrap();
    root
}

/// A project with its own sign-in: no starter account files.
fn own_sign_in(temporary: &Path, contract: bool) -> PathBuf {
    let root = temporary.canonicalize().unwrap();
    for directory in ["src/migrations", "src/controllers"] {
        fs::create_dir_all(root.join(directory)).unwrap();
    }
    fs::write(root.join("Cargo.toml"), MANIFEST).unwrap();
    fs::write(
        root.join("src/main.rs"),
        "mod auth_contract;\nmod controllers;\npub mod migrations;\n\n#[rullst::runtime::main]\nasync fn main() {\n    rullst::artisan!(crate::migrations::get_migrations());\n}\n",
    )
    .unwrap();
    fs::write(root.join("src/migrations/mod.rs"), "").unwrap();
    fs::write(root.join("src/controllers/mod.rs"), "pub mod home;\n").unwrap();
    if contract {
        fs::write(root.join("src/auth_contract.rs"), CONTRACT).unwrap();
    }
    root
}

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap()
}

fn features(root: &Path) -> Vec<String> {
    let manifest: toml::Value = toml::from_str(&read(root, "Cargo.toml")).unwrap();
    manifest["dependencies"]["rullst"]["features"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|feature| feature.as_str().map(str::to_owned))
        .collect()
}

#[test]
fn the_saas_starter_is_wired_end_to_end() {
    for hot in [false, true] {
        let temporary = tempfile::tempdir().unwrap();
        let root = saas(temporary.path(), hot);
        let planned = plan(&root, TIMESTAMP).unwrap();
        assert_eq!(planned.accounts, Accounts::Starter);
        writes::apply(&planned.edits).unwrap();

        for (path, source) in render_sources(Accounts::Starter) {
            assert_eq!(read(&root, path), source, "{path}");
        }
        assert!(
            read(&root, "src/controllers/mfa/account.rs")
                .contains("use crate::models::user::User;")
        );
        let migration = "m20261110120000_create_user_mfa_factors_table";
        assert!(
            read(&root, &format!("src/migrations/{migration}.rs"))
                .contains("CREATE TABLE mfa_challenges")
        );
        assert!(read(&root, "src/migrations/mod.rs").contains(&format!("pub mod {migration};")));
        let features = features(&root);
        for feature in ["orm", "security", "auth"] {
            assert!(features.iter().any(|known| known == feature), "{feature}");
        }
        assert!(read(&root, "src/controllers/mod.rs").contains("pub mod mfa;"));
        // The smoke test of the non-hot starter now checks the security page.
        if !hot {
            assert!(read(&root, "src/security_tests.rs").contains("(\"/account/security\", 303),"));
        }
        assert!(read(&root, "src/pages/mod.rs").contains("pub mod mfa;"));

        let login = read(&root, "src/controllers/auth_controller.rs");
        assert_eq!(
            login
                .matches("crate::controllers::mfa::second_step(user.id, \"/dashboard\")")
                .count(),
            1
        );
        let hook = login.find("mfa::second_step").unwrap();
        assert!(hook < login.find("match login_cookie(&user) {").unwrap());
        let router_path = if hot { "src/lib.rs" } else { "src/main.rs" };
        let router = read(&root, router_path);
        assert_eq!(
            router.matches("controllers::mfa::challenge_submit").count(),
            1
        );
        assert!(
            router.find("\"/login/mfa\"").unwrap()
                > router.find("\"/login\", rullst::routing::post").unwrap()
        );
        assert!(
            router.find("\"/account/security\"").unwrap() < router.find("csrf_middleware").unwrap()
        );
        let main = read(&root, "src/main.rs");
        let reset = main.find("controllers::mfa::admin::run().await").unwrap();
        assert!(reset < main.find("rullst::artisan!(").unwrap());
        assert!(
            read(&root, "src/pages/auth.rs")
                .contains("<a href=\"/account/security\" class=\"btn-nexus\">")
        );
        for path in [
            "src/controllers/auth_controller.rs",
            "src/main.rs",
            "src/pages/auth.rs",
            router_path,
        ] {
            syn::parse_file(&read(&root, path)).unwrap();
        }
        let notes = planned.notes.join("\n");
        for expected in [
            "db:migrate",
            "now sends accounts with a second factor to /login/mfa",
            "Mounted GET /account/security",
            "admin:mfa-reset <email>",
        ] {
            assert!(notes.contains(expected), "{expected}: {notes}");
        }
        assert!(!notes.contains("Mount the routes inside your CSRF layer"));

        // A second run refuses before writing anything.
        let error = plan(&root, TIMESTAMP).err().unwrap().to_string();
        assert!(error.starts_with("refusing to overwrite existing second-factor scaffold: src/controllers/mfa.rs, src/controllers/mfa/account.rs"), "{error}");
        assert!(error.contains("src/migrations/m20261110120000_create_user_mfa_factors_table.rs"));
    }
}

#[test]
fn an_application_with_its_own_sign_in_uses_the_contract() {
    let temporary = tempfile::tempdir().unwrap();
    let root = own_sign_in(temporary.path(), true);
    let planned = plan(&root, TIMESTAMP).unwrap();
    assert_eq!(planned.accounts, Accounts::Contract);
    writes::apply(&planned.edits).unwrap();
    assert_eq!(
        read(&root, "src/controllers/mfa/account.rs"),
        CONTRACT_ACCOUNT_TEMPLATE
    );
    let features = features(&root);
    assert!(features.iter().any(|feature| feature == "security"));
    assert!(!features.iter().any(|feature| feature == "auth"));
    // The pages module is created and declared next to the controllers.
    assert_eq!(read(&root, "src/pages/mod.rs"), "pub mod mfa;\n");
    assert_eq!(
        read(&root, "src/controllers/mod.rs"),
        "pub mod home;\npub mod mfa;\n"
    );
    let main = read(&root, "src/main.rs");
    assert!(main.contains("pub mod pages;"), "{main}");
    assert!(main.contains("if let Some(status) = controllers::mfa::admin::run().await {"));
    syn::parse_file(&main).unwrap();
    let notes = planned.notes.join("\n");
    assert!(
        notes.contains("each behind auth_contract::require_account"),
        "{notes}"
    );
    assert!(notes.contains("In your sign-in handler, right after the password check"));
    assert!(notes.contains("Link /account/security from your account pages."));
}

#[test]
fn a_project_without_accounts_names_both_ways_in() {
    let temporary = tempfile::tempdir().unwrap();
    let root = own_sign_in(temporary.path(), false);
    let error = plan(&root, TIMESTAMP).err().unwrap().to_string();
    assert_eq!(
        error,
        format!(
            "make:mfa needs the accounts of `cargo rullst auth` or the SaaS starter (this project lacks src/models/user.rs with a `User` model, `pub async fn auth_middleware` in src/middlewares/auth_middleware.rs), or, for an application with its own sign-in, the authentication contract in src/auth_contract.rs ({})",
            auth_contract::DOCS
        )
    );
    assert!(!root.join("src/controllers/mfa.rs").exists());

    // A starter model without `revoke_sessions` names just that.
    let temporary = tempfile::tempdir().unwrap();
    let root = saas(temporary.path(), false);
    let user = read(&root, "src/models/user.rs").replace("fn revoke_sessions(", "fn end_all(");
    fs::write(root.join("src/models/user.rs"), user).unwrap();
    let error = plan(&root, TIMESTAMP).err().unwrap().to_string();
    assert!(
        error
            .contains("(this project lacks `revoke_sessions` in the `User` of src/models/user.rs)"),
        "{error}"
    );
    // The contract is the way in for such a project.
    fs::write(root.join("src/auth_contract.rs"), CONTRACT).unwrap();
    let error = plan(&root, TIMESTAMP).err().unwrap().to_string();
    assert!(
        error.contains("does not declare `mod auth_contract;`"),
        "{error}"
    );
    let main =
        read(&root, "src/main.rs").replace("pub mod pages;", "pub mod pages;\nmod auth_contract;");
    fs::write(root.join("src/main.rs"), main).unwrap();
    assert_eq!(plan(&root, TIMESTAMP).unwrap().accounts, Accounts::Contract);
    assert!(plan(&root, "2026").is_err());
}

#[test]
fn the_password_sign_in_hook_goes_before_the_session_once() {
    let controller = crate::generators::auth::controllers::render_auth_controller(None);
    let hooked = edits::login_hook(&controller).unwrap().unwrap();
    let start = hooked.find("pub async fn login_submit").unwrap();
    let register = hooked.find("pub async fn register_submit").unwrap();
    let hook = hooked
        .find("    // Two-step verification (cargo rullst make:mfa): an account with a\n")
        .unwrap();
    assert!(start < hook && hook < register);
    assert!(hooked.contains("            return auth::login_page(&token, Some(error.message()), None, nonce).into_response();\n"));
    assert_eq!(edits::login_hook(&hooked).unwrap(), None);
    // The registration keeps signing in directly.
    assert_eq!(hooked[register..].matches("mfa::second_step").count(), 0);
    let unrecognized = controller.replace(
        "match login_cookie(&user) {\n        Ok(cookie) => redirect_with_cookie(\"/dashboard\"",
        "match other(&user) {\n        Ok(cookie) => redirect_with_cookie(\"/dashboard\"",
    );
    assert_eq!(edits::login_hook(&unrecognized).unwrap(), None);
}

#[test]
fn the_reset_command_runs_before_artisan_in_an_async_main() {
    let main = "pub mod controllers;\n\n#[rullst::runtime::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {\n    rullst::artisan!(crate::migrations::get_migrations());\n    Ok(())\n}\n";
    let hooked = edits::main_hook(main).unwrap().unwrap();
    assert!(hooked.contains("{\n    // `<application> admin:mfa-reset <email>` (cargo rullst make:mfa) removes an\n    // account's second factor and signs out its sessions, then exits.\n    if let Some(status) = controllers::mfa::admin::run().await {\n        std::process::exit(status);\n    }\n    rullst::artisan!("));
    assert_eq!(edits::main_hook(&hooked).unwrap(), None);
    assert_eq!(
        edits::main_hook(&main.replace("async fn", "fn")).unwrap(),
        None
    );
    assert_eq!(
        edits::main_hook(&main.replace("pub mod controllers;", "")).unwrap(),
        None
    );
    assert_eq!(
        edits::main_hook(&main.replace("rullst::artisan!", "other!")).unwrap(),
        None
    );
}

#[test]
fn the_dashboard_links_the_security_page_once() {
    let temporary = tempfile::tempdir().unwrap();
    let root = saas(temporary.path(), false);
    let page = read(&root, "src/pages/auth.rs");
    let linked = edits::dashboard_link(&page).unwrap();
    assert!(linked.contains("🔑 Change password</a>\n             <a href=\"/account/security\" class=\"btn-nexus\">🔐 Two-step verification</a>"));
    assert_eq!(edits::dashboard_link(&linked), None);
    assert_eq!(edits::dashboard_link("pub fn page() {}"), None);
    let router = read(&root, "src/main.rs");
    let routed = edits::router_routes(&router).unwrap().unwrap();
    assert_eq!(edits::router_routes(&routed).unwrap(), None);
    assert_eq!(
        edits::router_routes(&router.replace("fn router(", "fn routes(")).unwrap(),
        None
    );
}

#[test]
fn the_provider_sign_in_hook_clears_the_provider_cookie() {
    let flow =
        crate::generators::oauth::render_controller("google", "rullst::db::sqlx::Sqlite", false);
    let hooked = edits::oauth_hook(&flow).unwrap().unwrap();
    assert!(
        hooked.contains("    match crate::controllers::mfa::second_step(user.id, next).await {\n")
    );
    assert!(
        hooked.find("mfa::second_step").unwrap()
            < hooked
                .find("let session = match session_cookie(&user)")
                .unwrap()
    );
    syn::parse_file(&hooked).unwrap();
    assert_eq!(edits::oauth_hook(&hooked).unwrap(), None);
    // make:oauth after make:mfa writes the same lines.
    let generated =
        crate::generators::oauth::render_controller("google", "rullst::db::sqlx::Sqlite", true);
    assert_eq!(generated, hooked);
}

#[test]
fn generated_sources_are_panic_free_and_csp_strict() {
    for accounts in [Accounts::Starter, Accounts::Contract] {
        for (path, source) in render_sources(accounts) {
            syn::parse_file(source).unwrap();
            let production = source.split("#[cfg(test)]").next().unwrap();
            for forbidden in [
                ".unwrap(",
                ".expect(",
                "panic!(",
                "std::env::var(",
                "user-{",
                "\"Rullst\"",
                "<script",
                " style=\"",
                "rullst_security::",
                "use axum",
            ] {
                assert!(!production.contains(forbidden), "{path}: {forbidden}");
            }
        }
    }
    let migration = render_mfa_migration("m20261110120000_create_user_mfa_factors_table");
    syn::parse_file(&migration).unwrap();
    assert!(!migration.contains("__MIGRATION_NAME__"));
    let pages = PAGES_TEMPLATE;
    assert!(pages.contains("<style nonce={nonce}>{STYLES}</style>"));
    // The CSS is text inside html!: nothing the escaping would change.
    let styles = pages
        .split("const STYLES: &str = \"")
        .nth(1)
        .unwrap()
        .split("\";")
        .next()
        .unwrap();
    for escaped in ['<', '>', '&', '"', '\''] {
        assert!(!styles.contains(escaped), "{escaped}");
    }
    let factor = FACTOR_TEMPLATE;
    assert!(factor.contains("build_mfa_qr_svg(&issuer(), &account.email, &secret)"));
    assert!(factor.contains("[\"MFA_ISSUER\", \"APP_NAME\"]"));
    let challenge = CHALLENGE_TEMPLATE;
    assert!(challenge.contains("const CHALLENGE_SECONDS: i64 = 300;"));
    assert!(challenge.contains("pub const MAX_ATTEMPTS: i32 = 5;"));
    assert!(challenge.contains(".bind(sha256_hex(&id))"));
    assert!(CONTROLLER_TEMPLATE.contains("account::end_sessions(user_id)"));
}

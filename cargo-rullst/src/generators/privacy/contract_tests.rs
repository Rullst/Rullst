//! `make:privacy` for an application with its own sign-in, through its
//! authentication contract.
use super::*;
use std::fs;

fn saas(temporary: &Path) -> std::path::PathBuf {
    let root = files::symlinked_project(temporary);
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13\"\n",
    )
    .unwrap();
    crate::blueprints::apply(
        crate::blueprints::SAAS_BLUEPRINT_ID,
        &root,
        "consumer",
        "consumer",
        false,
        false,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .unwrap();
    root
}

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap()
}

#[test]
fn an_application_with_its_own_sign_in_uses_the_contract() {
    let temporary = tempfile::tempdir().unwrap();
    let root = saas(temporary.path());
    // Without the contract, a changed starter authentication names it.
    let middleware = read(&root, "src/middlewares/auth_middleware.rs");
    fs::write(
        root.join("src/middlewares/auth_middleware.rs"),
        format!("{middleware}\nfn other() {{}}\n"),
    )
    .unwrap();
    let error = plan(
        &root,
        None,
        "saas",
        "greeting-v1",
        3600,
        "tenant-alpha",
        None,
    )
    .err()
    .unwrap()
    .to_string();
    assert!(
        error.ends_with(&format!(
            "differs from the starter's. An application with its own sign-in can use the authentication contract in src/auth_contract.rs instead ({})",
            auth_contract::DOCS
        )),
        "{error}"
    );
    fs::write(root.join("src/middlewares/auth_middleware.rs"), middleware).unwrap();

    auth_contract::own_sign_in(&root);
    let planned = plan(
        &root,
        None,
        "saas",
        "greeting-v1",
        3600,
        "tenant-alpha",
        None,
    )
    .unwrap();
    assert!(planned.notes.is_empty(), "{:?}", planned.notes);
    files::apply(&planned.edits).unwrap();
    let controller = read(&root, "src/controllers/privacy_controller.rs");
    assert!(controller.contains("crate::auth_contract::require_account"));
    assert!(!controller.contains("auth_middleware"));
    let proof = read(&root, "src/controllers/privacy/proof.rs");
    assert!(proof.contains("crate::auth_contract::session_binding(headers)"));
    assert!(!proof.contains("extract_session_cookie"));
    let profile = read(&root, "src/controllers/privacy/profile.rs");
    assert!(profile.contains("crate::auth_contract::find_account(user_id)"));
    assert!(!profile.contains("FROM users"));
    assert!(read(&root, "src/main.rs").contains("privacy_controller::routes()"));
}

#[test]
fn a_router_without_the_csrf_boundary_is_left_to_the_application() {
    let temporary = tempfile::tempdir().unwrap();
    let root = saas(temporary.path());
    auth_contract::own_sign_in(&root);
    let main = read(&root, "src/main.rs").replace(
        ".layer(rullst::server::from_fn(rullst::security::csrf_middleware))",
        ".layer(from_fn(rullst::security::csrf_middleware))",
    );
    fs::write(root.join("src/main.rs"), &main).unwrap();
    let planned = plan(
        &root,
        None,
        "saas",
        "greeting-v1",
        3600,
        "tenant-alpha",
        None,
    )
    .unwrap();
    assert_eq!(planned.notes.len(), 1);
    assert!(planned.notes[0].starts_with("Mount the routes inside your CSRF layer"));
    files::apply(&planned.edits).unwrap();
    assert_eq!(read(&root, "src/main.rs"), main);
    // A contract that lacks what privacy calls is refused.
    let temporary = tempfile::tempdir().unwrap();
    let root = saas(temporary.path());
    auth_contract::own_sign_in(&root);
    let contract = read(&root, "src/auth_contract.rs")
        .replace("pub fn session_binding(", "fn session_binding(");
    fs::write(root.join("src/auth_contract.rs"), contract).unwrap();
    let error = plan(
        &root,
        None,
        "saas",
        "greeting-v1",
        3600,
        "tenant-alpha",
        None,
    )
    .err()
    .unwrap()
    .to_string();
    assert!(error.contains("lacks `pub fn session_binding`"), "{error}");
}

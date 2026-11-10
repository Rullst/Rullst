//! Detection of the authentication contract in `src/auth_contract.rs`.
use super::*;
use std::fs;

const REFERENCE: &str = include_str!("reference.rs.template");

fn project(contract: &str, main: &str) -> tempfile::TempDir {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), main).unwrap();
    fs::write(root.join(PATH), contract).unwrap();
    temporary
}

const MAIN: &str = "mod auth_contract;\nmod controllers;\nfn main() {}\n";

fn refusal(root: &Path, needs: Needs) -> String {
    check(root, needs)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default()
}

#[test]
fn the_reference_contract_satisfies_every_generator() {
    let temporary = project(REFERENCE, MAIN);
    let root = temporary.path();
    assert!(present(root).unwrap());
    for needs in [MFA, PRIVACY, OAUTH] {
        check(root, needs).unwrap();
    }
    syn::parse_file(REFERENCE).unwrap();
}

#[test]
fn each_generator_names_only_the_functions_it_lacks() {
    let without =
        |name: &str| REFERENCE.replace(&format!("fn {name}("), &format!("fn {name}_renamed("));
    let temporary = project(&without("session_binding"), MAIN);
    let root = temporary.path();
    // MFA and OAuth do not call it; privacy does.
    check(root, MFA).unwrap();
    check(root, OAUTH).unwrap();
    assert_eq!(
        refusal(root, PRIVACY),
        format!(
            "make:privacy uses this application's own sign-in through src/auth_contract.rs, which lacks `pub fn session_binding` ({DOCS})"
        )
    );

    fs::write(root.join(PATH), without("create_account")).unwrap();
    check(root, MFA).unwrap();
    assert!(refusal(root, OAUTH).contains("lacks `pub async fn create_account` ("));

    fs::write(
        root.join(PATH),
        without("require_account").replace("pub async fn end_sessions", "pub fn end_sessions"),
    )
    .unwrap();
    let error = refusal(root, MFA);
    assert!(
        error.contains("lacks `pub async fn require_account`, `pub async fn end_sessions` ("),
        "{error}"
    );
    check(root, OAUTH).unwrap();

    // A private function does not count.
    fs::write(
        root.join(PATH),
        REFERENCE.replace("pub async fn find_account(", "async fn find_account("),
    )
    .unwrap();
    assert!(refusal(root, PRIVACY).contains("lacks `pub async fn find_account` ("));
}

#[test]
fn the_account_type_needs_the_public_fields_each_generator_reads() {
    let temporary = project(
        &REFERENCE.replace("    pub name: String,", "    name: String,"),
        MAIN,
    );
    let root = temporary.path();
    check(root, MFA).unwrap();
    check(root, OAUTH).unwrap();
    assert!(refusal(root, PRIVACY).contains("lacks the `pub name` field of `ContractAccount` ("));

    fs::write(
        root.join(PATH),
        REFERENCE.replace("pub struct ContractAccount", "pub struct Account"),
    )
    .unwrap();
    assert!(refusal(root, MFA).contains("lacks `pub struct ContractAccount` ("));
    fs::write(
        root.join(PATH),
        REFERENCE.replace("pub struct ContractAccount", "struct ContractAccount"),
    )
    .unwrap();
    assert!(refusal(root, MFA).contains("lacks `pub struct ContractAccount` ("));
    let tuple = REFERENCE.replace(
        "pub struct ContractAccount {\n    pub id: i32,",
        "pub struct ContractAccount(pub i32);\npub struct Unused {\n    pub id: i32,",
    );
    fs::write(root.join(PATH), tuple).unwrap();
    let error = refusal(root, MFA);
    assert!(error.contains("lacks the `pub id` field of `ContractAccount`, the `pub email` field of `ContractAccount` ("), "{error}");
}

#[test]
fn every_crate_root_that_compiles_the_controllers_declares_the_module() {
    let temporary = project(REFERENCE, "mod controllers;\nfn main() {}\n");
    let root = temporary.path();
    assert_eq!(
        refusal(root, MFA),
        "make:mfa: src/main.rs compiles the controllers but does not declare `mod auth_contract;` (src/auth_contract.rs)"
    );
    // A hot-reload starter compiles them in the library and the binary.
    fs::write(
        root.join("src/lib.rs"),
        "pub mod auth_contract;\npub mod controllers;\n",
    )
    .unwrap();
    assert!(refusal(root, MFA).starts_with("make:mfa: src/main.rs compiles"));
    fs::write(root.join("src/main.rs"), MAIN).unwrap();
    check(root, MFA).unwrap();
    // Without a controllers module yet, the crate root must declare it.
    fs::write(root.join("src/lib.rs"), "pub fn router() {}\n").unwrap();
    fs::write(
        root.join("src/main.rs"),
        "mod auth_contract;\nfn main() {}\n",
    )
    .unwrap();
    assert_eq!(
        refusal(root, MFA),
        "make:mfa: src/lib.rs compiles the controllers but does not declare `mod auth_contract;` (src/auth_contract.rs)"
    );
    fs::write(root.join("src/lib.rs"), "mod auth_contract;\n").unwrap();
    check(root, MFA).unwrap();
}

#[test]
fn an_unreadable_or_invalid_contract_is_reported() {
    let temporary = project("fn broken(", MAIN);
    let root = temporary.path();
    assert_eq!(
        refusal(root, OAUTH),
        "make:oauth: src/auth_contract.rs is not valid Rust"
    );
    fs::remove_file(root.join(PATH)).unwrap();
    assert!(!present(root).unwrap());
    assert!(refusal(root, OAUTH).starts_with("make:oauth could not read src/auth_contract.rs: "));
    let message = neither(PRIVACY, "src/models/user.rs is missing").to_string();
    assert_eq!(
        message,
        format!(
            "make:privacy needs the accounts of `cargo rullst auth` or the SaaS starter (src/models/user.rs is missing), or, for an application with its own sign-in, the authentication contract in src/auth_contract.rs ({DOCS})"
        )
    );
}

#[test]
fn signatures_name_async_functions() {
    assert_eq!(
        REQUIRE_ACCOUNT.signature(),
        "`pub async fn require_account`"
    );
    assert_eq!(SESSION_BINDING.signature(), "`pub fn session_binding`");
    assert_eq!(MFA.functions.len(), 5);
    assert_eq!(
        PRIVACY.functions,
        [REQUIRE_ACCOUNT, FIND_ACCOUNT, SESSION_BINDING]
    );
    assert_eq!(OAUTH.fields, ["id", "email"]);
}

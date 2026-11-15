//! The 13.0.0-alpha.6 passkeys feature: code that names Rullst's
//! `auth::passkey` must enable it, unless a manifest of the workspace does.

use super::super::files::manifest_findings;
use super::files::plan;
use super::*;

const PASSKEYS: &str = "V13-AUTH-PASSKEYS-FEATURE";
const PACKAGE: &str = "[package]\nname = \"app\"\n\n";

const SOURCE: &str = r#"
use rullst::auth::passkey::{PasskeyAuth, PasskeyConfig};
use rullst_auth::passkey::shared::SharedPasskeyAuth;
use rullst::auth::hash_password;
fn auth() -> rullst::auth::passkey::PasskeyAuth {
    let config = rullst_auth::auth::passkey::PasskeyConfig::new("App", "a.example", "https://a.example");
    PasskeyAuth::new(&config).unwrap()
}
fn handle(_: &rullst::auth::auth::passkey::Passkey) {}
"#;

fn found(source: &str, facts: &WorkspaceFacts) -> Vec<usize> {
    scan_with(source, "lib.rs", facts)
        .into_iter()
        .filter(|(code, _)| *code == PASSKEYS)
        .map(|(_, line)| line)
        .collect()
}

fn facts(manifest: &str) -> WorkspaceFacts {
    manifest_findings(&[plan(PathBuf::from("Cargo.toml"), manifest)]).1
}

#[test]
fn passkey_paths_must_change_when_no_manifest_builds_passkeys() {
    assert_eq!(found(SOURCE, &WorkspaceFacts::default()), [2, 3, 5, 6, 9]);
    let manifest = format!(
        "{PACKAGE}[dependencies]\nrullst = {{ version = \"=13.0.0-alpha.6\", features = [\"auth\"] }}\n"
    );
    assert_eq!(found(SOURCE, &facts(&manifest)), [2, 3, 5, 6, 9]);
}

#[test]
fn a_manifest_feature_that_builds_passkeys_suppresses_the_rule() {
    let dependencies = [
        r#"rullst = { version = "=13.0.0-alpha.6", features = ["auth-passkeys"] }"#,
        r#"rullst = { version = "=13.0.0-alpha.6", features = ["orm", "auth-sqlite"] }"#,
        r#"rullst = { version = "=13.0.0-alpha.6", features = ["auth-passkey-postgres"] }"#,
        r#"rullst-auth = { version = "=13.0.0-alpha.6", features = ["passkeys"] }"#,
        r#"rullst-auth = { version = "=13.0.0-alpha.6", features = ["sqlite"] }"#,
        r#"rullst-auth = { version = "=13.0.0-alpha.6", features = ["passkey-postgres"] }"#,
        r#"login = { package = "rullst-auth", version = "=13.0.0-alpha.6", features = ["passkeys"] }"#,
    ];
    for dependency in dependencies {
        let manifest = format!("{PACKAGE}[dependencies]\n{dependency}\n");
        let facts = facts(&manifest);
        assert!(facts.passkeys, "{dependency}");
        assert_eq!(found(SOURCE, &facts), Vec::<usize>::new(), "{dependency}");
    }
    let workspace = "[workspace.dependencies]\nrullst-auth = { version = \"=13.0.0-alpha.6\", features = [\"passkeys\"] }\n";
    assert!(facts(workspace).passkeys);
    for reference in [
        "rullst/auth-passkeys",
        "rullst?/auth-passkeys",
        "rullst/auth-sqlite",
    ] {
        let manifest = format!(
            "{PACKAGE}[dependencies]\nrullst = {{ version = \"=13.0.0-alpha.6\", optional = true, features = [\"auth\"] }}\n\n[features]\nlogin = [\"{reference}\"]\n"
        );
        assert!(facts(&manifest).passkeys, "{reference}");
    }
}

#[test]
fn a_manifest_may_enable_the_passkeys_more_than_once() {
    let manifest = format!(
        "{PACKAGE}[dependencies]\nrullst = {{ version = \"=13.0.0-alpha.6\", features = [\"auth-passkeys\"] }}\nrullst-auth = {{ version = \"=13.0.0-alpha.6\", features = [\"passkeys\"] }}\n\n[features]\nlogin = [\"rullst/auth-passkeys\"]\n"
    );
    assert!(facts(&manifest).passkeys);
    let twice = format!(
        "{PACKAGE}[dependencies]\nrullst = {{ version = \"=13.0.0-alpha.6\", features = [\"auth-passkeys\"] }}\n\n[features]\nlogin = [\"rullst/auth-passkeys\"]\n"
    );
    assert!(facts(&twice).passkeys);
}

#[test]
fn features_that_do_not_build_passkeys_keep_the_rule() {
    let manifests = [
        r#"rullst = { version = "=13.0.0-alpha.6", features = ["auth", "auth-jwt", "auth-sessions-sqlite", "nexus"] }"#,
        r#"rullst-auth = { version = "=13.0.0-alpha.6", features = ["jwt", "recovery-sqlite"] }"#,
        r#"other = { version = "1", features = ["passkeys", "auth-passkeys"] }"#,
        "rullst = { version = \"=13.0.0-alpha.6\", optional = true }\n\n[features]\nlogin = [\"rullst/auth\", \"other/auth-passkeys\"]",
    ];
    for dependency in manifests {
        let manifest = format!("{PACKAGE}[dependencies]\n{dependency}\n");
        assert!(!facts(&manifest).passkeys, "{dependency}");
    }
}

#[test]
fn own_passkey_modules_comments_and_strings_do_not_match() {
    let own = r#"
//! Not `rullst::auth::passkey::PasskeyAuth`.
mod auth { pub mod passkey { pub struct Key; } }
use crate::auth::passkey::Key;
const TEXT: &str = "rullst_auth::passkey::PasskeyAuth";
fn key(_: my_app::auth::passkey::Key, _: self::auth::passkey::Key) {}
"#;
    assert_eq!(found(own, &WorkspaceFacts::default()), Vec::<usize>::new());
    let bare = "mod auth { pub mod passkey {} }\nuse auth::passkey;\n";
    assert_eq!(found(bare, &WorkspaceFacts::default()), Vec::<usize>::new());
    let imported =
        "use rullst::auth;\nfn f() { let _ = auth::passkey::PasskeyConfig::default(); }\n";
    assert_eq!(found(imported, &WorkspaceFacts::default()), [2]);
    let direct = "extern crate rullst_auth;\nuse auth::passkey::Passkey;\n";
    assert_eq!(found(direct, &WorkspaceFacts::default()), [2]);
}

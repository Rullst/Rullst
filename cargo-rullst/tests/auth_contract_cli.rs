//! An application with its own sign-in: `make:oauth`, `make:mfa` and
//! `make:privacy` generate against its authentication contract
//! (`src/auth_contract.rs`), and the application with all three compiles
//! without warnings.
use cargo_rullst::{blueprints, generators::project::cargo_toml::build_cargo_toml};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn success(output: Output) -> String {
    let text = format!(
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{text}");
    text
}

fn rullst(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(root)
        .args(args)
        .env("RULLST_DISABLE_UPDATE_CHECK", "1")
        .env("NO_COLOR", "1")
        .output()
        .unwrap()
}

/// Every `.rs` file under `directory`.
fn sources(directory: &Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn generated_code_compiles_against_the_authentication_contract() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let manifest = build_cargo_toml(
        "contract-consumer",
        false,
        true,
        "Sqlite",
        &[],
        false,
        false,
        blueprints::SAAS_BLUEPRINT_ID,
        "Zero-Bundle HTMX",
        workspace,
    )
    .unwrap();
    fs::write(root.join("Cargo.toml"), manifest).unwrap();
    fs::copy(workspace.join("Cargo.lock"), root.join("Cargo.lock")).unwrap();
    blueprints::apply(
        blueprints::SAAS_BLUEPRINT_ID,
        root,
        "contract-consumer",
        "contract_consumer",
        false,
        false,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .unwrap();

    // The application's own sign-in: its account model lives elsewhere, so
    // no generator recognizes the starter's, and the contract reaches it.
    fs::rename(
        root.join("src/models/user.rs"),
        root.join("src/models/account.rs"),
    )
    .unwrap();
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    for path in files {
        let source = fs::read_to_string(&path).unwrap();
        let moved = source
            .replace("models::user::", "models::account::")
            .replace("pub mod user;", "pub mod account;");
        if moved != source {
            fs::write(&path, moved).unwrap();
        }
    }
    let contract = include_str!("../src/generators/auth_contract/reference.rs.template")
        .replace("crate::models::user::User", "crate::models::account::User");
    fs::write(root.join("src/auth_contract.rs"), contract).unwrap();
    let main = fs::read_to_string(root.join("src/main.rs")).unwrap();
    fs::write(
        root.join("src/main.rs"),
        main.replacen(
            "pub mod migrations;",
            "pub mod auth_contract;\npub mod migrations;",
            1,
        ),
    )
    .unwrap();

    let oauth = success(rullst(root, &["make:oauth", "google"]));
    assert!(oauth.contains("Mount the routes yourself"), "{oauth}");
    assert!(
        oauth.contains("controllers::oauth::credential_rate_limit"),
        "{oauth}"
    );
    let flow = fs::read_to_string(root.join("src/controllers/oauth/mod.rs")).unwrap();
    assert!(flow.contains("auth_contract::create_account(&mut transaction, email, &name)"));
    assert!(!flow.contains("crate::models::user"));

    let mfa = success(rullst(root, &["make:mfa"]));
    assert!(
        mfa.contains("for this application's sign-in (src/auth_contract.rs)"),
        "{mfa}"
    );
    assert!(
        mfa.contains("each behind auth_contract::require_account"),
        "{mfa}"
    );
    assert!(
        mfa.contains("Provider sign-in (make:oauth) also asks for the code"),
        "{mfa}"
    );
    let flow = fs::read_to_string(root.join("src/controllers/oauth/mod.rs")).unwrap();
    assert!(flow.contains("crate::controllers::mfa::second_step(user.id, next)"));

    let privacy_source = workspace.join("rullst-privacy");
    let privacy = success(rullst(
        root,
        &[
            "make:privacy",
            "--tenant-ref",
            "tenant-alpha",
            "--purpose-version",
            "greeting-v1",
            "--validity-seconds",
            "3600",
            "--privacy-source",
            privacy_source.to_str().unwrap(),
        ],
    ));
    assert!(
        privacy.contains("Privacy choices installed at /privacy"),
        "{privacy}"
    );
    let controller =
        fs::read_to_string(root.join("src/controllers/privacy_controller.rs")).unwrap();
    assert!(controller.contains("crate::auth_contract::require_account"));
    assert!(
        fs::read_to_string(root.join("src/main.rs"))
            .unwrap()
            .contains("privacy_controller::routes()")
    );

    let output = Command::new("cargo")
        .current_dir(root)
        .args(["clippy", "--all-targets", "-j", "1", "--", "-D", "warnings"])
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_INCREMENTAL", "0")
        .output()
        .unwrap();
    success(output);
    let _ = Command::new("cargo")
        .current_dir(root)
        .args(["clean", "--package", "contract-consumer"])
        .env("CARGO_TARGET_DIR", workspace.join("target"))
        .output();
}

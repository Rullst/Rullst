//! `make:oauth` on real generated projects: Google, GitHub, Microsoft and
//! Apple sign-in in one project run end to end through the generated router
//! with the offline providers, and the command refuses projects without the
//! account model.
use cargo_rullst::{blueprints, generators::project::cargo_toml::build_cargo_toml};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn rullst(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn generated_sign_in_with_every_provider_runs_end_to_end_offline() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let mut manifest = build_cargo_toml(
        "oauth-consumer",
        true,
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
    manifest.push_str("\n[dev-dependencies]\ntower = { version = \"0.5\", features = [\"util\"] }\n\n[profile.test]\ndebug = 0\nincremental = false\n");
    fs::write(root.join("Cargo.toml"), manifest).unwrap();
    fs::copy(workspace.join("Cargo.lock"), root.join("Cargo.lock")).unwrap();
    blueprints::apply(
        blueprints::SAAS_BLUEPRINT_ID,
        root,
        "oauth-consumer",
        "oauth_consumer",
        false,
        true,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .unwrap();
    for provider in ["google", "github", "microsoft", "apple"] {
        let installed = success(rullst(root, &["make:oauth", provider]));
        assert!(String::from_utf8_lossy(&installed.stdout).contains("--features oauth-mock"));
    }
    // A second run changes nothing.
    let before = fs::read(root.join("Cargo.toml")).unwrap();
    assert!(!rullst(root, &["make:oauth", "github"]).status.success());
    assert_eq!(before, fs::read(root.join("Cargo.toml")).unwrap());

    fs::write(
        root.join("src/controllers/oauth/oauth_flow_contract.rs"),
        include_str!("fixtures/oauth_flow_contract.rs"),
    )
    .unwrap();
    let path = root.join("src/controllers/oauth/mod.rs");
    let mut source = fs::read_to_string(&path).unwrap();
    source.push_str(
        "\n#[cfg(test)]\n#[path = \"oauth_flow_contract.rs\"]\nmod oauth_flow_contract;\n",
    );
    fs::write(path, source).unwrap();
    // The hot-reload starter compiles controllers in the library and binary
    // test targets; both use the generated library router.
    let path = root.join("src/main.rs");
    let mut source = fs::read_to_string(&path).unwrap();
    source.push_str("\n#[cfg(test)]\nuse oauth_consumer::router;\n");
    fs::write(path, source).unwrap();

    let app_key = (0..4)
        .map(|_| format!("{:016x}", rand::random::<u64>()))
        .collect::<String>();
    let cargo = |args: &[&str]| {
        let mut command = Command::new("cargo");
        command
            .current_dir(root)
            .args(args)
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true")
            .env("CARGO_PROFILE_DEV_DEBUG", "0")
            .env("APP_KEY", &app_key)
            .env("RULLST_ENV", "development")
            .env("GOOGLE_CLIENT_ID", "mock_google_client")
            .env("GOOGLE_CLIENT_SECRET", "mock_google_secret")
            .env_remove("GOOGLE_REDIRECT_URL")
            .env("GITHUB_CLIENT_ID", "mock_github")
            .env("GITHUB_CLIENT_SECRET", "mock_github")
            .env_remove("GITHUB_REDIRECT_URL")
            .env("MICROSOFT_TENANT", "common")
            .env("MICROSOFT_CLIENT_ID", "mock_microsoft")
            .env("MICROSOFT_CLIENT_SECRET", "mock_microsoft")
            .env_remove("MICROSOFT_REDIRECT_URL")
            .env("APPLE_CLIENT_ID", "mock_apple")
            .env("APPLE_TEAM_ID", "mock_apple")
            .env("APPLE_KEY_ID", "mock_apple")
            .env("APPLE_PRIVATE_KEY", "mock_apple")
            .env_remove("APPLE_REDIRECT_URL")
            .env_remove("RULLST_PUBLIC_ORIGIN")
            .env("BILLING_API_KEY", "mock_contract")
            .env("BILLING_WEBHOOK_SECRET", "mock_contract")
            .env("BILLING_PROVIDER", "stripe");
        command
    };
    // With the oauth-mock feature the whole browser flow runs offline.
    let flow = success(
        cargo(&[
            "test",
            "--lib",
            "--features",
            "oauth-mock",
            "generated_oauth_",
            "-j",
            "1",
            "--",
            "--test-threads=1",
        ])
        .output()
        .unwrap(),
    );
    let stdout = String::from_utf8_lossy(&flow.stdout);
    for test in [
        "generated_oauth_flow_contract",
        "generated_oauth_configuration_contract",
    ] {
        assert!(
            stdout.contains(&format!(
                "test controllers::oauth::oauth_flow_contract::{test} ... ok"
            )),
            "{test} did not run:\n{stdout}"
        );
    }
    // Without it, mock credentials stop the application at startup.
    let refused = success(
        cargo(&[
            "test",
            "--lib",
            "generated_oauth_configuration_contract",
            "-j",
            "1",
        ])
        .output()
        .unwrap(),
    );
    assert!(
        String::from_utf8_lossy(&refused.stdout)
            .contains("generated_oauth_configuration_contract ... ok")
    );
    success(
        cargo(&[
            "clippy",
            "--all-targets",
            "--features",
            "oauth-mock",
            "-j",
            "1",
            "--",
            "-D",
            "warnings",
        ])
        .output()
        .unwrap(),
    );
    success(
        cargo(&["clean", "--package", "oauth-consumer"])
            .output()
            .unwrap(),
    );
}

#[test]
fn make_oauth_needs_the_account_model_and_composes_with_make_auth() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"plain\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = { version = \"13.0.0-alpha.2\", default-features = false, features = [\"orm\", \"strict-sqlite\"] }\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("src/migrations")).unwrap();
    fs::write(
        root.join("src/main.rs"),
        "pub mod migrations;\n\nfn main() {\n    rullst::artisan!(crate::migrations::get_migrations());\n}\n",
    )
    .unwrap();
    fs::write(root.join("src/migrations/mod.rs"), "").unwrap();
    let refused = rullst(root, &["make:oauth", "google"]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("account model"));
    assert!(!root.join("src/controllers/oauth").exists());

    success(rullst(root, &["auth"]));
    let installed = success(rullst(root, &["make:oauth", "google"]));
    // make:auth leaves the router to the application, so the routes are an
    // instruction; its login page gets the link.
    assert!(String::from_utf8_lossy(&installed.stdout).contains("Mount the routes yourself"));
    let page = fs::read_to_string(root.join("src/pages/auth.rs")).unwrap();
    assert!(page.contains("{crate::controllers::oauth::google::login_link()}"));
    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("\"oauth\"") && manifest.contains("\"oauth-sqlite\""));
    assert!(manifest.contains("oauth-mock = [\"rullst/oauth-mock\"]"));
    // A second provider adds its module, link and, for Apple's form POST
    // callback, the CSRF exemption in a new Rullst.toml.
    let apple = success(rullst(root, &["make:oauth", "apple"]));
    let stdout = String::from_utf8_lossy(&apple.stdout);
    assert!(stdout.contains("POST /auth/apple/callback to controllers::oauth::apple::callback"));
    assert!(!stdout.contains("db:migrate"));
    let config = fs::read_to_string(root.join("Rullst.toml")).unwrap();
    assert!(config.contains("csrf_signed_webhook_paths = [\"/auth/apple/callback\"]"));
    let page = fs::read_to_string(root.join("src/pages/auth.rs")).unwrap();
    assert!(page.contains("{crate::controllers::oauth::apple::login_link()}"));
    for generated in [
        "src/controllers/oauth/mod.rs",
        "src/controllers/oauth/google.rs",
        "src/controllers/oauth/apple.rs",
        "src/models/oauth_identity.rs",
    ] {
        syn::parse_file(&fs::read_to_string(root.join(generated)).unwrap()).unwrap();
    }
}

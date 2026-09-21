//! Real generated SaaS/auth/database with an explicit test-only provider adapter.
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

#[test]
fn generated_saas_authorizes_only_current_bound_subscriptions() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let mut manifest = build_cargo_toml(
        "entitlement-consumer",
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
        "entitlement-consumer",
        "entitlement_consumer",
        false,
        true,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .unwrap();
    fs::write(
        root.join("src/controllers/billing_entitlement_contract.rs"),
        include_str!("fixtures/billing_entitlement_contract.rs"),
    )
    .unwrap();
    let path = root.join("src/controllers/billing_live.rs");
    let mut source = fs::read_to_string(&path).unwrap();
    source.push_str("\n#[cfg(test)]\n#[path = \"billing_entitlement_contract.rs\"]\nmod entitlement_contract;\n");
    fs::write(path, source).unwrap();
    // The hot-reload blueprint compiles controllers in both the library and
    // binary test targets. Both must use the actual generated library router.
    let path = root.join("src/main.rs");
    let mut source = fs::read_to_string(&path).unwrap();
    source.push_str("\n#[cfg(test)]\nuse entitlement_consumer::router;\n");
    fs::write(path, source).unwrap();
    let app_key = (0..4)
        .map(|_| format!("{:016x}", rand::random::<u64>()))
        .collect::<String>();
    for plans in [
        Some("price_pro"),
        None,
        Some("price_unknown"),
        Some("price_pro,price_pro"),
    ] {
        let mut command = Command::new("cargo");
        command
            .current_dir(root)
            .args([
                "test",
                "--lib",
                "generated_entitlement_http_and_revision_contract",
                "-j",
                "1",
                "--",
                "--nocapture",
            ])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true")
            .env("CARGO_PROFILE_DEV_DEBUG", "0")
            .env("APP_KEY", &app_key)
            .env("RULLST_ENV", "development")
            .env("BILLING_API_KEY", "mock_contract")
            .env("BILLING_WEBHOOK_SECRET", "mock_contract")
            .env("BILLING_PROVIDER", "stripe")
            .env("BILLING_ACCOUNT_ID", "acct_contract")
            .env("BILLING_ALLOWED_PLAN_IDS", "price_pro,price_basic")
            .env_remove("BILLING_REPORT_PLAN_IDS");
        if let Some(plans) = plans {
            command.env("BILLING_REPORT_PLAN_IDS", plans);
        }
        eprintln!("generated entitlement report policy: {plans:?}");
        success(command.output().unwrap());
    }
    success(
        Command::new("cargo")
            .current_dir(root)
            .args(["clippy", "--all-targets", "-j", "1", "--", "-D", "warnings"])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true")
            .env("CARGO_PROFILE_DEV_DEBUG", "0")
            .output()
            .unwrap(),
    );
    success(
        Command::new("cargo")
            .current_dir(root)
            .args(["clean", "--package", "entitlement-consumer"])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true")
            .output()
            .unwrap(),
    );
}

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(command: &mut Command, action: &str) -> Output {
    command
        .output()
        .unwrap_or_else(|error| panic!("could not {action}: {error}"))
}

fn assert_success(output: &Output, action: &str) {
    assert!(
        output.status.success(),
        "{action} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn clean_generated_package(project: &Path, workspace: &Path, package_name: &str) {
    let cleaned = run(
        Command::new("cargo")
            .current_dir(project)
            .args(["clean", "--package", package_name])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true"),
        "clean generated billing package",
    );
    assert_success(&cleaned, "generated billing package cleanup");
}

fn install_workspace_lock(project: &Path, workspace: &Path) {
    fs::copy(workspace.join("Cargo.lock"), project.join("Cargo.lock"))
        .expect("copy workspace lockfile into generated billing project");
}

fn billing_migration(project: &Path) -> PathBuf {
    fs::read_dir(project.join("src/migrations"))
        .expect("migration directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("_create_subscriptions_table.rs"))
        })
        .expect("billing migration")
}

fn contract_source(database: &str) -> String {
    let initialize = if database == "turso" {
        r#"let config = rullst::orm::polyglot::TursoConfig::new("mock_billing", "")
        .with_offline_path("turso-development.db")?;
    rullst::orm::polyglot::TursoOrm::init(config).await?;"#
    } else {
        r#"rullst::orm::Orm::init("sqlite://db.sqlite?mode=rwc").await?;"#
    };
    format!(
        r#"#![allow(dead_code)]

#[path = "../controllers/mod.rs"]
mod controllers;
#[path = "../models/mod.rs"]
mod models;
#[path = "../pages/mod.rs"]
mod pages;

use models::billing_customer::BillingCustomer;
use models::subscription::Subscription;
use rullst::capital::{{SubscriptionStatus, WebhookEvent}};
use rullst::server::{{Extension, StatusCode}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    {initialize}
    let mut owner = BillingCustomer {{
        id: 0,
        workspace_id: 7,
        email: "owner@example.com".to_string(),
        customer_id: None,
        created_at: String::new(),
        updated_at: String::new(),
    }};
    owner.save().await?;

    let event = WebhookEvent {{
        subscription_id: "sub_contract".to_string(),
        customer_id: "cus_owner".to_string(),
        customer_email: owner.email.clone(),
        plan_id: "price_pro".to_string(),
        status: SubscriptionStatus::Active,
        ends_at: None,
    }};
    let response = controllers::billing_controller::webhook_handler(Extension(event)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let subscription = Subscription::find_by_subscription_id("sub_contract")
        .await?
        .ok_or("subscription was not persisted")?;
    assert_eq!(subscription.workspace_id, 7);
    assert_eq!(subscription.customer_id, "cus_owner");

    let mut other = BillingCustomer {{
        id: 0,
        workspace_id: 9,
        email: "other@example.com".to_string(),
        customer_id: None,
        created_at: String::new(),
        updated_at: String::new(),
    }};
    other.save().await?;
    let unknown_plan = WebhookEvent {{
        subscription_id: "sub_unknown".to_string(),
        customer_id: "cus_other".to_string(),
        customer_email: other.email.clone(),
        plan_id: "price_unknown".to_string(),
        status: SubscriptionStatus::Active,
        ends_at: None,
    }};
    let response = controllers::billing_controller::webhook_handler(Extension(unknown_plan)).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let forged_owner = WebhookEvent {{
        subscription_id: "sub_contract".to_string(),
        customer_id: "cus_other".to_string(),
        customer_email: other.email.clone(),
        plan_id: "price_pro".to_string(),
        status: SubscriptionStatus::Active,
        ends_at: None,
    }};
    let response = controllers::billing_controller::webhook_handler(Extension(forged_owner)).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let other = BillingCustomer::find_by_email("other@example.com")
        .await?
        .ok_or("other customer disappeared")?;
    assert!(other.customer_id.is_none());
    Ok(())
}}
"#
    )
}

fn verify_backend(database: &str) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let project = std::env::temp_dir().join(format!(
        "rullst-billing-scaffold-{database}-{}",
        rand::random::<u64>()
    ));
    let cli = env!("CARGO_BIN_EXE_rullst");
    let generated = run(
        Command::new(cli)
            .current_dir(workspace)
            .arg("new")
            .arg(&project)
            .args([
                "--default",
                "--api",
                "--database",
                database,
                "--skip-initial-migration",
            ]),
        "generate base project",
    );
    assert_success(&generated, "base project generation");

    let scaffolded = run(
        Command::new(cli)
            .current_dir(&project)
            .args(["make:billing", "--model", "Workspace"]),
        "scaffold billing",
    );
    assert_success(&scaffolded, "billing generation");

    let manifest = fs::read_to_string(project.join("Cargo.toml")).expect("generated manifest");
    let parsed: toml::Value = toml::from_str(&manifest).expect("valid generated manifest");
    let package_name = parsed["package"]["name"]
        .as_str()
        .expect("generated package name")
        .to_string();
    let features = parsed["dependencies"]["rullst"]["features"]
        .as_array()
        .expect("rullst features");
    for required in ["orm", "capital"] {
        assert_eq!(
            features
                .iter()
                .filter(|feature| feature.as_str() == Some(required))
                .count(),
            1,
            "feature {required} must be enabled exactly once"
        );
    }

    let model =
        fs::read_to_string(project.join("src/models/subscription.rs")).expect("subscription model");
    let controller = fs::read_to_string(project.join("src/controllers/billing_controller.rs"))
        .expect("billing controller");
    let migration_path = billing_migration(&project);
    let migration = fs::read_to_string(&migration_path).expect("billing migration");
    assert_eq!(model.contains("backend = \"turso\""), database == "turso");
    assert!(controller.contains("find_by_subscription_id"));
    assert!(!controller.contains("#[derive(Debug)]\nstruct BillingConfig"));
    assert!(migration.contains("subscriptions_subscription_id_unique"));

    fs::create_dir_all(project.join("src/bin")).expect("contract bin directory");
    fs::write(
        project.join("src/bin/billing_contract.rs"),
        contract_source(database),
    )
    .expect("write billing runtime contract");
    install_workspace_lock(&project, workspace);

    let checked = run(
        Command::new("cargo")
            .current_dir(&project)
            .args(["clippy", "--all-targets", "--", "-D", "warnings"])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true"),
        "Clippy generated billing project",
    );
    assert_success(&checked, "generated billing project Clippy");

    let migrated = run(
        Command::new("cargo")
            .current_dir(&project)
            .args(["run", "--quiet", "--bin", &package_name, "--", "db:migrate"])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true"),
        "run generated billing migrations",
    );
    assert_success(&migrated, "generated billing migrations");

    let runtime = run(
        Command::new("cargo")
            .current_dir(&project)
            .args(["run", "--quiet", "--bin", "billing_contract"])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true"),
        "run generated billing contract",
    );
    assert_success(&runtime, "generated billing runtime contract");

    let controller_before = controller;
    let duplicate = run(
        Command::new(cli)
            .current_dir(&project)
            .args(["make:billing", "--model", "Workspace"]),
        "rerun billing generator",
    );
    assert!(!duplicate.status.success(), "rerun must fail closed");
    assert_eq!(
        fs::read_to_string(project.join("src/controllers/billing_controller.rs"))
            .expect("preserved controller"),
        controller_before
    );
    assert_eq!(
        fs::read_dir(project.join("src/migrations"))
            .expect("migration directory")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with("_create_subscriptions_table.rs")
            })
            .count(),
        1
    );

    clean_generated_package(&project, workspace, &package_name);
    fs::remove_dir_all(&project).expect("remove generated project");
}

#[test]
// TM-DEPLOY-06: materialized billing defaults compile and deny cross-owner mutation.
fn billing_scaffold_compiles_migrates_enforces_ownership_and_refuses_collisions() {
    for database in ["sqlite", "turso"] {
        verify_backend(database);
    }
}

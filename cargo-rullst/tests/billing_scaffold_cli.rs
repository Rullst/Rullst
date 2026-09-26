#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

#[path = "billing_scaffold_support/paddle.rs"]
mod paddle_support;

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

fn target_directory(workspace: &Path) -> PathBuf {
    match std::env::var_os("CARGO_TARGET_DIR").filter(|value| !value.is_empty()) {
        Some(value) => workspace.join(value),
        None => workspace.join("target"),
    }
}

fn clean_generated_package(project: &Path, workspace: &Path, package_name: &str) {
    let cleaned = run(
        Command::new("cargo")
            .current_dir(project)
            .args(["clean", "--package", package_name])
            .env("CARGO_TARGET_DIR", target_directory(workspace))
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
    let execute_sql = if database == "turso" {
        r#"rullst::orm::polyglot::TursoOrm::store()?.execute(
            rullst::orm::polyglot::TursoStatement::new(statement, vec![])?
        ).await?;"#
    } else {
        r#"rullst::db::sqlx::query(rullst::db::sqlx::AssertSqlSafe(statement)).execute(rullst::db::Orm::pool()?).await?;"#
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
use rullst::server::{{Body, Extension, Form, Request, StatusCode}};
use rullst::web::axum::Router;
use tower::ServiceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    use controllers::billing_controller::{{BillingIdentity, CheckoutForm}};
    let identity = BillingIdentity {{ owner_id: 7, email: "owner@example.com".into() }};
    if std::env::var_os("BILLING_CONTRACT_REAL_CREDENTIALS").is_some() {{
        // No pool is initialized: all real/mixed paths must stop before SQL/HTTP.
        let response = controllers::billing_controller::portal_redirect(Extension(identity.clone())).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let response = controllers::billing_controller::checkout_redirect(
            Extension(identity), Form(CheckoutForm {{ plan: "price_pro".into() }})
        ).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let event = WebhookEvent {{ subscription_id: "sub_fixture".into(), customer_id: "cus_fixture".into(),
            customer_email: "owner@example.com".into(), plan_id: "price_pro".into(),
            status: SubscriptionStatus::Active, ends_at: None }};
        let response = controllers::billing_controller::webhook_handler(Extension(event)).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let router = Router::new().route("/webhook", rullst::routing::post(|| async {{ StatusCode::IM_A_TEAPOT }}))
            .route_layer(rullst::server::from_fn(controllers::billing_controller::verify_billing_webhook));
        let response = router.oneshot(Request::builder().method("POST").uri("/webhook").body(Body::empty())?).await?;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        return Ok(());
    }}
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

    let checkout = controllers::billing_controller::checkout_redirect(
        Extension(identity), Form(CheckoutForm {{ plan: "price_pro".into() }})
    ).await;
    assert_eq!(checkout.status(), StatusCode::SEE_OTHER);
    assert!(checkout.headers()["location"].to_str()?.starts_with("https://checkout.stripe.com/"));

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

    // Abort either statement in the real generated transaction. Both rows must
    // remain unchanged, and retrying after the failure must remain possible.
    for (index, table, operation) in [(0, "billing_customers", "UPDATE"), (1, "subscriptions", "INSERT")] {{
        let email = format!("rollback{{index}}@example.com");
        let owner_id = 20 + index;
        let mut customer = BillingCustomer {{
            id: 0, workspace_id: owner_id, email: email.clone(), customer_id: None,
            created_at: String::new(), updated_at: String::new(),
        }};
        customer.save().await?;
        execute_sql(&format!("CREATE TRIGGER reject_billing_update BEFORE {{operation}} ON {{table}} BEGIN SELECT RAISE(ABORT, 'injected billing failure'); END")).await?;
        let event = WebhookEvent {{
            subscription_id: format!("sub_rollback{{index}}"), customer_id: format!("cus_rollback{{index}}"),
            customer_email: email.clone(), plan_id: "price_pro".into(),
            status: SubscriptionStatus::Active, ends_at: None,
        }};
        let failed = controllers::billing_controller::webhook_handler(Extension(event.clone())).await;
        assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let unchanged = BillingCustomer::find_by_email(&email).await?.ok_or("customer disappeared")?;
        assert!(unchanged.customer_id.is_none(), "customer binding survived rollback");
        assert!(Subscription::find_by_subscription_id(&event.subscription_id).await?.is_none(), "subscription survived rollback");
        execute_sql("DROP TRIGGER reject_billing_update").await?;
        let retried = controllers::billing_controller::webhook_handler(Extension(event)).await;
        assert_eq!(retried.status(), StatusCode::OK);
    }}
    Ok(())
}}

async fn execute_sql(statement: &str) -> Result<(), Box<dyn std::error::Error>> {{
    {execute_sql}
    Ok(())
}}
"#
    )
}

fn install_live_contract(project: &Path, database: &str) {
    let initialize = if database == "turso" {
        r#"let turso_config = rullst::orm::polyglot::TursoConfig::new("mock_billing", "")
            .with_offline_path("turso-development.db").unwrap();
        rullst::orm::polyglot::TursoOrm::init(turso_config).await.unwrap();"#
    } else {
        r#"rullst::orm::Orm::init("sqlite://db.sqlite?mode=rwc").await.unwrap();"#
    };
    let execute = if database == "turso" {
        r#"rullst::orm::polyglot::TursoOrm::store().unwrap().execute(
            rullst::orm::polyglot::TursoStatement::new(statement, vec![]).unwrap()).await.unwrap();"#
    } else {
        r#"rullst::db::sqlx::query(rullst::db::sqlx::AssertSqlSafe(statement))
        .execute(rullst::db::Orm::pool().unwrap()).await.unwrap();"#
    };
    let source = include_str!("fixtures/billing_live_contract.rs")
        .replace("__INITIALIZE_DB__", initialize)
        .replace("__EXECUTE_SQL__", execute);
    fs::write(
        project.join("src/controllers/billing_live_contract.rs"),
        source,
    )
    .unwrap();
    let path = project.join("src/controllers/billing_live.rs");
    let mut live = fs::read_to_string(&path).unwrap();
    live.push_str("\n#[cfg(test)]\n#[path = \"billing_live_contract.rs\"]\nmod live_contract;\n");
    fs::write(path, live).unwrap();
    paddle_support::install(project, database, initialize, execute);
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
    assert!(parsed["dependencies"].get("tower").is_none());
    fs::write(
        project.join("Cargo.toml"),
        manifest.replacen(
            "[dependencies]",
            "[dependencies]\ntower = { version = \"0.5\", features = [\"util\"] }",
            1,
        ),
    )
    .expect("install route-test dependency");
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
    install_live_contract(&project, database);

    let checked = run(
        Command::new("cargo")
            .current_dir(&project)
            .args(["clippy", "--all-targets", "--", "-D", "warnings"])
            .env("CARGO_TARGET_DIR", target_directory(workspace))
            .env("CARGO_NET_OFFLINE", "true"),
        "Clippy generated billing project",
    );
    assert_success(&checked, "generated billing project Clippy");

    let migrated = run(
        Command::new("cargo")
            .current_dir(&project)
            .args(["run", "--quiet", "--bin", &package_name, "--", "db:migrate"])
            .env("CARGO_TARGET_DIR", target_directory(workspace))
            .env("CARGO_NET_OFFLINE", "true"),
        "run generated billing migrations",
    );
    assert_success(&migrated, "generated billing migrations");

    for restart in [false, true] {
        let mut command = Command::new("cargo");
        command
            .current_dir(&project)
            .args([
                "test",
                "--quiet",
                "--bin",
                "billing_contract",
                "durable_live_billing_contract",
            ])
            .env("BILLING_ACCOUNT_ID", "acct_contract")
            .env("CARGO_TARGET_DIR", target_directory(workspace))
            .env("CARGO_NET_OFFLINE", "true");
        if restart {
            command.env("BILLING_RESTART_CHECK", "1");
        }
        assert_success(
            &run(&mut command, "exercise durable billing and restart"),
            "live billing state contract",
        );
    }

    paddle_support::verify(&project, workspace);

    for acknowledgement in [None, Some("yes"), Some("I_UNDERSTAND_REAL_CHARGES")] {
        let mut command = Command::new("cargo");
        command
            .current_dir(&project)
            .args([
                "test",
                "--quiet",
                "--bin",
                "billing_contract",
                "real_money_activation_requires_explicit_acknowledgement",
            ])
            .env("BILLING_ACCOUNT_ID", "acct_contract")
            .env_remove("BILLING_LIVE_ACKNOWLEDGEMENT")
            .env("CARGO_TARGET_DIR", target_directory(workspace))
            .env("CARGO_NET_OFFLINE", "true");
        if let Some(value) = acknowledgement {
            command.env("BILLING_LIVE_ACKNOWLEDGEMENT", value);
        }
        assert_success(
            &run(&mut command, "validate real-money activation gate"),
            "real-money activation gate",
        );
    }

    let runtime = run(
        Command::new("cargo")
            .current_dir(&project)
            .args(["run", "--quiet", "--bin", "billing_contract"])
            .env("RULLST_ENV", "development")
            .env("BILLING_PROVIDER", "stripe")
            .env("BILLING_API_KEY", "mock_key")
            .env("BILLING_WEBHOOK_SECRET", "mock_webhook")
            .env("BILLING_ALLOWED_PLAN_IDS", "price_pro")
            .env(
                "BILLING_REDIRECT_URL",
                "https://app.example.invalid/dashboard",
            )
            .env("CARGO_TARGET_DIR", target_directory(workspace))
            .env("CARGO_NET_OFFLINE", "true"),
        "run generated billing contract",
    );
    assert_success(&runtime, "generated billing runtime contract");

    for provider in ["stripe", "lemonsqueezy", "paddle"] {
        for (api_key, webhook_secret) in [
            ("fixture_invalid_live_credential", "mock_webhook"),
            ("mock_key", "fixture_real_webhook_secret_0123456789"),
            (
                "fixture_invalid_live_credential",
                "fixture_real_webhook_secret_0123456789",
            ),
            ("MOCK_uppercase_is_not_a_provider_mock", "mock_webhook"),
        ] {
            let real_billing = run(
                Command::new("cargo")
                    .current_dir(&project)
                    .args(["run", "--quiet", "--bin", "billing_contract"])
                    .env("RULLST_ENV", "development")
                    .env("BILLING_PROVIDER", provider)
                    .env("BILLING_STORE_ID", "42")
                    .env("BILLING_API_KEY", api_key)
                    .env("BILLING_WEBHOOK_SECRET", webhook_secret)
                    .env("BILLING_ALLOWED_PLAN_IDS", "price_pro")
                    .env("BILLING_CONTRACT_REAL_CREDENTIALS", "1")
                    .env("CARGO_TARGET_DIR", target_directory(workspace))
                    .env("CARGO_NET_OFFLINE", "true"),
                "reject real generated billing before I/O",
            );
            assert_success(&real_billing, "real generated billing containment");
        }
    }

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

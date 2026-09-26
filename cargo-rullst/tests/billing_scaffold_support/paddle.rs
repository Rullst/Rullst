use super::{assert_success, run, target_directory};
use std::{fs, path::Path, process::Command};

pub(super) fn install(project: &Path, database: &str, initialize: &str, execute: &str) {
    let source = include_str!("../fixtures/billing_paddle_contract.rs")
        .replace("__INITIALIZE_DB__", initialize)
        .replace("__EXECUTE_SQL__", execute)
        .replace(
            "__OWNER_ID_TYPE__",
            if database == "turso" { "i64" } else { "i32" },
        );
    fs::write(
        project.join("src/controllers/billing_paddle_contract.rs"),
        source,
    )
    .unwrap();
    fs::write(
        project.join("src/controllers/billing_paddle_fake.rs"),
        include_str!("../fixtures/billing_paddle_fake.rs"),
    )
    .unwrap();
    let path = project.join("src/controllers/billing_paddle.rs");
    let mut paddle = fs::read_to_string(&path).unwrap();
    paddle.push_str(
        "\n#[cfg(test)]\n#[path = \"billing_paddle_contract.rs\"]\nmod paddle_contract;\n",
    );
    fs::write(path, paddle).unwrap();
}

pub(super) fn verify(project: &Path, workspace: &Path) {
    for restart in [false, true] {
        let mut command = Command::new("cargo");
        command
            .current_dir(project)
            .args([
                "test",
                "--quiet",
                "--bin",
                "billing_contract",
                "durable_paddle_billing_contract",
            ])
            .env("BILLING_ACCOUNT_ID", "acct_contract")
            .env("BILLING_PADDLE_ENVIRONMENT", "sandbox")
            .env("BILLING_PADDLE_PAYMENT_LINK", "https://app.example/pay")
            .env(
                "BILLING_REPORT_PLAN_IDS",
                "pri_00000000000000000000000001,pri_00000000000000000000000002",
            )
            .env("RULLST_ENV", "development")
            .env("CARGO_TARGET_DIR", target_directory(workspace))
            .env("CARGO_NET_OFFLINE", "true");
        if restart {
            command.env("BILLING_RESTART_CHECK", "1");
        }
        assert_success(
            &run(&mut command, "exercise durable Paddle state and restart"),
            "Paddle state contract",
        );
    }
    for acknowledgement in [None, Some("yes"), Some("I_UNDERSTAND_REAL_CHARGES")] {
        let mut command = Command::new("cargo");
        command
            .current_dir(project)
            .args([
                "test",
                "--quiet",
                "--bin",
                "billing_contract",
                "paddle_activation_gate_requires_exact_acknowledgement",
            ])
            .env("BILLING_ACCOUNT_ID", "acct_contract")
            .env("BILLING_PADDLE_ENVIRONMENT", "live")
            .env("BILLING_PADDLE_PAYMENT_LINK", "https://app.example/pay")
            .env_remove("BILLING_LIVE_ACKNOWLEDGEMENT")
            .env("CARGO_TARGET_DIR", target_directory(workspace))
            .env("CARGO_NET_OFFLINE", "true");
        if let Some(value) = acknowledgement {
            command.env("BILLING_LIVE_ACKNOWLEDGEMENT", value);
        }
        assert_success(
            &run(&mut command, "verify Paddle real-money activation gate"),
            "Paddle real-money activation gate",
        );
    }
}

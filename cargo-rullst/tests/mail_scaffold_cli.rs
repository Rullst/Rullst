#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;
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

#[test]
fn every_mail_scaffold_compiles_escapes_html_and_fails_closed() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let project =
        std::env::temp_dir().join(format!("rullst-mail-scaffold-{}", rand::random::<u64>()));
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
                "sqlite",
                "--skip-initial-migration",
            ]),
        "generate base project",
    );
    assert_success(&generated, "base project generation");

    for arguments in [
        &["make:mail", "WelcomeEmail", "--welcome"][..],
        &["make:mail", "PasswordReset", "--reset"][..],
        &["make:mail", "OtpVerification", "--otp"][..],
        &["make:mail", "InvoiceReceipt", "--invoice"][..],
        &["make:mail", "CustomNotice"][..],
    ] {
        let scaffolded = run(
            Command::new(cli).current_dir(&project).args(arguments),
            "scaffold mailable",
        );
        assert_success(&scaffolded, "mailable generation");
    }

    let manifest = fs::read_to_string(project.join("Cargo.toml")).expect("generated manifest");
    let parsed: toml::Value = toml::from_str(&manifest).expect("valid generated manifest");
    let features = parsed["dependencies"]["rullst"]["features"]
        .as_array()
        .expect("rullst feature array");
    assert_eq!(
        features
            .iter()
            .filter(|feature| feature.as_str() == Some("mailer"))
            .count(),
        1,
        "mailer feature must be enabled exactly once"
    );

    let registry = fs::read_to_string(project.join("src/mail/mod.rs")).expect("mail registry");
    for module in [
        "welcome_email",
        "password_reset",
        "otp_verification",
        "invoice_receipt",
        "custom_notice",
    ] {
        assert!(registry.contains(&format!("pub mod {module};")));
    }
    assert!(
        fs::read_to_string(project.join("src/main.rs"))
            .expect("project root")
            .contains("pub mod mail;")
    );

    fs::create_dir_all(project.join("src/bin")).expect("contract bin directory");
    fs::write(
        project.join("src/bin/mail_contract.rs"),
        r##"#[allow(dead_code)]
#[path = "../mail/mod.rs"]
mod mail;

use mail::{CustomNotice, InvoiceReceipt, OtpVerification, PasswordReset, WelcomeEmail};

fn assert_escaped(html: &str) {
    assert!(!html.contains("<script>"));
    assert!(!html.contains("<img"));
    assert!(!html.contains("onclick="));
    assert!(html.contains("&lt;script&gt;") || html.contains("&lt;img"));
}

fn main() {
    let hostile = "<script>alert(1)</script>";
    let hostile_url = "https://example.com/\"><img src=x onerror=alert(1)>";

    let welcome = WelcomeEmail::new(
        "user@example.com",
        hostile,
        hostile_url,
        "https://example.com/unsubscribe",
    );
    assert_escaped(welcome.build().body_html.as_deref().expect("welcome HTML"));

    let reset = PasswordReset::new("user@example.com", hostile, hostile_url, 15);
    assert_escaped(reset.build().body_html.as_deref().expect("reset HTML"));

    let otp = OtpVerification::new("user@example.com", hostile, 5);
    assert_escaped(otp.build().body_html.as_deref().expect("OTP HTML"));

    let invoice = InvoiceReceipt::new(
        "user@example.com",
        hostile,
        hostile,
        hostile,
        hostile_url,
    );
    assert_escaped(invoice.build().body_html.as_deref().expect("invoice HTML"));

    let custom = CustomNotice::new("user@example.com", hostile, hostile);
    assert_escaped(custom.build().body_html.as_deref().expect("custom HTML"));
}
"##,
    )
    .expect("write generated mail contract");

    let checked = run(
        Command::new("cargo")
            .current_dir(&project)
            .args(["clippy", "--all-targets", "--", "-D", "warnings"])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true"),
        "Clippy generated mail project",
    );
    assert_success(&checked, "generated mail project Clippy");

    let runtime = run(
        Command::new("cargo")
            .current_dir(&project)
            .args(["run", "--quiet", "--bin", "mail_contract"])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true"),
        "run generated mail escaping contract",
    );
    assert_success(&runtime, "generated mail escaping contract");

    let welcome_path = project.join("src/mail/welcome_email.rs");
    let welcome_before = fs::read_to_string(&welcome_path).expect("welcome mailable");
    let duplicate = run(
        Command::new(cli)
            .current_dir(&project)
            .args(["make:mail", "WelcomeEmail", "--welcome"]),
        "rerun mail generator",
    );
    assert!(!duplicate.status.success(), "rerun must fail closed");
    assert_eq!(
        fs::read_to_string(&welcome_path).expect("preserved mailable"),
        welcome_before
    );

    let traversal = run(
        Command::new(cli)
            .current_dir(&project)
            .args(["make:mail", "../../escape"]),
        "try unsafe mailable name",
    );
    assert!(!traversal.status.success(), "unsafe name must be rejected");
    assert!(!project.join("escape.rs").exists());

    let conflicting = run(
        Command::new(cli).current_dir(&project).args([
            "make:mail",
            "AmbiguousMail",
            "--welcome",
            "--otp",
        ]),
        "try conflicting mail variants",
    );
    assert!(
        !conflicting.status.success(),
        "conflicting variants must be rejected"
    );
    assert!(!project.join("src/mail/ambiguous_mail.rs").exists());

    fs::remove_dir_all(&project).expect("remove generated project");
}

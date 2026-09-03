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

fn clean_generated_package(project: &Path, workspace: &Path, package_name: &str) {
    let cleaned = run(
        Command::new("cargo")
            .current_dir(project)
            .args(["clean", "--package", package_name])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true"),
        "clean generated mail package",
    );
    assert_success(&cleaned, "generated mail package cleanup");
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
        &["make:mail-invoice"][..],
        &["make:mail-dunning"][..],
    ] {
        let scaffolded = run(
            Command::new(cli).current_dir(&project).args(arguments),
            "scaffold mailable",
        );
        assert_success(&scaffolded, "mailable generation");
    }

    let manifest = fs::read_to_string(project.join("Cargo.toml")).expect("generated manifest");
    let parsed: toml::Value = toml::from_str(&manifest).expect("valid generated manifest");
    let package_name = parsed["package"]["name"]
        .as_str()
        .expect("generated package name")
        .to_string();
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
    assert_eq!(
        features
            .iter()
            .filter(|feature| feature.as_str() == Some("capital"))
            .count(),
        1,
        "capital feature must be enabled exactly once for the fiscal template"
    );

    let registry = fs::read_to_string(project.join("src/mail/mod.rs")).expect("mail registry");
    for module in [
        "welcome_email",
        "password_reset",
        "otp_verification",
        "invoice_receipt",
        "custom_notice",
        "fiscal_invoice_email",
        "payment_dunning_email",
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

use mail::{
    CustomNotice, FiscalInvoiceEmail, InvoiceReceipt, OtpVerification, PasswordReset,
    PaymentDunningEmail, WelcomeEmail,
};
use mail::payment_dunning_email::DunningStage;
use rullst::capital::fiscal::{
    FiscalCertificate, FiscalEmitter, FiscalResponseKind, NfseEnvironment, NfseNationalClient,
    TaxRegime,
};

fn assert_escaped(html: &str) {
    assert!(!html.contains("<script>"));
    assert!(!html.contains("<img"));
    assert!(!html.contains("onclick="));
    assert!(html.contains("&lt;script&gt;") || html.contains("&lt;img"));
}

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let emitter = FiscalEmitter {
        cnpj: "11.222.333/0001-81".to_string(),
        inscricao_municipal: "12345".to_string(),
        legal_name: "Rullst Serviços Ltda".to_string(),
        trade_name: None,
        ibge_code: "3550308".to_string(),
        tax_regime: TaxRegime::SimplesNacional,
    };
    let fiscal_client = NfseNationalClient::new(
        emitter,
        FiscalCertificate::offline_mock(),
        NfseEnvironment::Mock,
    );
    let offline_response = fiscal_client.transmit_dps("<DPS/>").await?;
    let fiscal = FiscalInvoiceEmail::from_nfse_response(
        "user@example.com",
        hostile,
        hostile,
        &offline_response,
    )?
    .with_document_url("https://example.com/preview")?;
    let fiscal_message = fiscal.build()?;
    assert!(fiscal_message.subject.contains("NOT AUTHORIZED"));
    assert_escaped(
        fiscal_message
            .body_html
            .as_deref()
            .expect("fiscal preview HTML"),
    );

    let mut contradictory_response = offline_response.clone();
    contradictory_response.kind = FiscalResponseKind::OfficialAuthorization;
    assert!(FiscalInvoiceEmail::from_nfse_response(
        "user@example.com",
        "Customer",
        "BRL 10.00",
        &contradictory_response,
    )
    .is_err());

    let mut official_response = offline_response.clone();
    official_response.kind = FiscalResponseKind::OfficialAuthorization;
    official_response.nfse_number = 42;
    official_response.access_key = "35260811222333000181000000000000000000000000000000".to_string();
    let official = FiscalInvoiceEmail::from_nfse_response(
        "user@example.com",
        "Customer",
        "BRL 10.00",
        &official_response,
    )?;
    let official_message = official.build()?;
    assert_eq!(official_message.subject, "NFS-e #42 issued");
    assert!(!official_message.subject.contains("NOT AUTHORIZED"));

    let international = FiscalInvoiceEmail::international_receipt(
        "user@example.com",
        hostile,
        "receipt-42",
        hostile,
    )?;
    let international_message = international.build()?;
    assert!(international_message.subject.contains("receipt-42"));
    assert_escaped(
        international_message
            .body_html
            .as_deref()
            .expect("international receipt HTML"),
    );

    for stage in [
        DunningStage::GentleReminder,
        DunningStage::ActionRequired,
        DunningStage::ServicePaused,
    ] {
        let dunning = PaymentDunningEmail::new(
            "user@example.com",
            hostile,
            "invoice-42",
            hostile,
            stage,
        )?
        .with_billing_url("https://example.com/billing")?;
        let dunning_message = dunning.build()?;
        assert!(dunning_message.subject.contains("invoice-42"));
        assert_escaped(
            dunning_message
                .body_html
                .as_deref()
                .expect("dunning HTML"),
        );
    }

    let dangerous_link = PaymentDunningEmail::new(
        "user@example.com",
        "Customer",
        "invoice-43",
        "USD 10.00",
        DunningStage::ActionRequired,
    )?;
    assert!(
        dangerous_link
            .with_billing_url("javascript:alert(1)")
            .is_err()
    );

    Ok(())
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

    let fiscal_path = project.join("src/mail/fiscal_invoice_email.rs");
    let fiscal_before = fs::read_to_string(&fiscal_path).expect("fiscal mailable");
    let duplicate_fiscal = run(
        Command::new(cli)
            .current_dir(&project)
            .arg("make:mail-invoice"),
        "rerun fiscal mail generator",
    );
    assert!(
        !duplicate_fiscal.status.success(),
        "fiscal rerun must fail closed"
    );
    assert_eq!(
        fs::read_to_string(&fiscal_path).expect("preserved fiscal mailable"),
        fiscal_before
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

    clean_generated_package(&project, workspace, &package_name);
    fs::remove_dir_all(&project).expect("remove generated project");
}

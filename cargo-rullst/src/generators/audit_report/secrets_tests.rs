#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::generators::audit_report::tests::project;

/// Secret-shaped fixtures built at run time, so no literal token is committed.
fn fixtures() -> Vec<(&'static str, String)> {
    vec![
        (
            "private key header",
            format!("-----BEGIN {} PRIVATE KEY-----", "RSA"),
        ),
        (
            "AWS access key ID",
            format!("{}{}", "AKIA", "Q7W3E9R2T5Y8U1I4"),
        ),
        (
            "Stripe live secret key",
            ["sk", "live", "Z9y8X7w6V5u4T3s2R1q0"].join("_"),
        ),
        (
            "GitHub personal access token",
            format!("{}_{}", "ghp", "a1B2".repeat(9)),
        ),
        (
            "GitHub fine-grained token",
            format!("{}_{}", "github_pat", "11AAAA_bbbb".repeat(3)),
        ),
        (
            "Slack token",
            format!("{}-{}", "xoxb", "1234567890-abcdefABCDEF"),
        ),
    ]
}

#[test]
fn every_high_signal_pattern_is_found_with_only_a_redacted_preview() {
    for (kind, secret) in fixtures() {
        let content = format!("let value = \"{secret}\";\n");
        let found = scan_text("src/config.rs", Path::new("src/config.rs"), &content).unwrap();
        assert_eq!(found.len(), 1, "{kind}");
        let finding = &found[0];
        assert_eq!(finding.message, format!("{kind} in a tracked file"));
        assert_eq!(finding.location(), "src/config.rs:1");
        let preview = finding.preview.as_deref().unwrap();
        assert_eq!(preview.chars().count(), 5, "{kind}");
        assert!(preview.ends_with('…'));
        assert!(!finding.message.contains(&secret) && !preview.contains(&secret));
    }
    let clean = "let key = std::env::var(\"STRIPE_SECRET_KEY\")?; // sk_test_ placeholder\n";
    assert!(
        scan_text("src/a.rs", Path::new("src/a.rs"), clean)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn committed_env_files_flag_long_secret_and_key_values_only() {
    let long = "Zq8".repeat(8);
    let content = format!(
        "# APP_SECRET={long}\nAPP_SECRET={long}\nexport API_KEY=\"{long}\"\nSHORT_KEY=abc\nDATABASE_URL={long}\nPLACEHOLDER_KEY=change-me-to-a-long-random-value\n"
    );
    let found = scan_text(".env", Path::new(".env"), &content).unwrap();
    let locations: Vec<_> = found.iter().map(Finding::location).collect();
    assert_eq!(locations, [".env:2", ".env:3"]);
    assert!(found[1].message.contains("`API_KEY`"));
    assert_eq!(found[0].preview.as_deref(), Some("Zq8Z…"));
    for template in [".env.example", "config/.env.sample"] {
        assert!(
            scan_text(template, Path::new(template), &content)
                .unwrap()
                .is_empty()
        );
    }
    assert_eq!(
        scan_text(".env.production", Path::new(".env.production"), &content)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn check_skips_target_binaries_and_reports_missing_git_as_not_checked() {
    let secret = fixtures()[1].1.clone();
    let root = project(&[
        ("src/keys.rs", &format!("const K: &str = \"{secret}\";\n")),
        ("target/debug/keys.rs", &format!("{secret}\n")),
        ("clean.txt", "nothing here\n"),
    ]);
    std::fs::write(root.path().join("blob.bin"), [0u8, 1, 2, b'A']).unwrap();
    let tracked = [
        "src/keys.rs",
        "target/debug/keys.rs",
        "clean.txt",
        "blob.bin",
        "missing.rs",
    ]
    .map(PathBuf::from)
    .to_vec();
    let check = check(root.path(), Ok(tracked));
    assert_eq!(check.status, EvidenceStatus::Findings(1));
    assert_eq!(check.findings[0].location(), "src/keys.rs:1");
    assert!(
        check
            .detail
            .starts_with("2 tracked text file(s) scanned; 2 skipped")
    );

    let clean = super::check(root.path(), Ok(vec![PathBuf::from("clean.txt")]));
    assert_eq!(clean.status, EvidenceStatus::NoFindings);

    let missing = tracked_files(OsStr::new("rullst-missing-git-fixture"), root.path());
    let check = super::check(root.path(), missing);
    assert!(matches!(check.status, EvidenceStatus::NotChecked(_)));
    assert!(!check.status.fails());
}

#[test]
fn git_ls_files_lists_only_tracked_files() {
    let git = OsStr::new("git");
    if Command::new(git).arg("--version").output().is_err() {
        return;
    }
    let root = project(&[
        ("tracked.rs", "fn a() {}\n"),
        ("untracked.rs", "fn b() {}\n"),
    ]);
    let run = |arguments: &[&str]| {
        Command::new(git)
            .args(arguments)
            .current_dir(root.path())
            .output()
            .unwrap()
            .status
            .success()
    };
    assert!(run(&["init", "-q"]));
    assert!(run(&["add", "tracked.rs"]));
    assert_eq!(
        tracked_files(git, root.path()).unwrap(),
        [PathBuf::from("tracked.rs")]
    );
    let outside = tempfile::tempdir().unwrap();
    // Not a work tree (unless the temporary directory is inside one).
    if let Err(reason) = tracked_files(git, outside.path()) {
        assert!(reason.contains("not a Git work tree"));
    }
}

#[test]
fn redaction_keeps_four_characters() {
    assert_eq!(redact("abcdefgh"), "abcd…");
    assert_eq!(redact("ab"), "ab…");
    assert_eq!(redact("çãõéxyz"), "çãõé…");
}

#[test]
fn redaction_removes_every_high_signal_value_and_counts_it() {
    let long = "Zq8".repeat(8);
    for (kind, secret) in fixtures() {
        let text = format!("+let value = \"{secret}\";\nkeep this line");
        let (redacted, count) = redact_secrets(&text).unwrap();
        assert_eq!(count, 1, "{kind}");
        assert!(!redacted.contains(&secret), "{kind}");
        assert!(redacted.contains(&format!("[redacted: {kind}]")), "{kind}");
        assert!(redacted.ends_with("\nkeep this line"));
    }
    let env = format!("STRIPE_SECRET_KEY={long}\nexport API_KEY=\"{long}\"\nAPP_KEY=change-me");
    let (redacted, count) = redact_secrets(&env).unwrap();
    assert_eq!(count, 2);
    assert!(!redacted.contains(&long));
    assert!(
        redacted.contains("STRIPE_SECRET_KEY=[redacted]") && redacted.contains("APP_KEY=change-me")
    );
    let url = format!("DATABASE_URL=postgres://app:{}@db:5432/app", "pw".repeat(3));
    let (redacted, count) = redact_secrets(&url).unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        redacted,
        "DATABASE_URL=postgres://app:[redacted]@db:5432/app"
    );
    assert_eq!(
        redact_secrets("plain\ntext").unwrap(),
        ("plain\ntext".to_string(), 0)
    );
}

#[test]
fn variable_references_angle_placeholders_and_short_values_stay_visible() {
    for line in [
        "PAYMENT_GATEWAY_KEY=${PAYMENT_GATEWAY_TOKEN_VALUE}",
        "PAYMENT_GATEWAY_KEY=<PAYMENT_GATEWAY_TOKEN_VALUE>",
        "API_KEY=abc123",
    ] {
        assert_eq!(
            redact_secrets(line).unwrap(),
            (line.to_string(), 0),
            "{line}"
        );
    }
}

#[test]
fn the_tracked_file_bound_is_inclusive() {
    let root = project(&[("clean.txt", "nothing here\n")]);
    let at_bound: Vec<PathBuf> = (0..MAX_TRACKED_FILES)
        .map(|index| PathBuf::from(format!("missing-{index}.rs")))
        .collect();
    let mut over_bound = at_bound.clone();
    over_bound.push(PathBuf::from("clean.txt"));
    assert_eq!(
        check(root.path(), Ok(at_bound)).status,
        EvidenceStatus::NoFindings
    );
    assert!(matches!(
        check(root.path(), Ok(over_bound)).status,
        EvidenceStatus::Error(_)
    ));
}

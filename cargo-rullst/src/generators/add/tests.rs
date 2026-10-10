use super::capability::{CAPABILITIES, Capability, EnvEntry, NAMES, Snippet, find};
use super::plan::{AddError, assigned_keys, dotenv_change, env_example_change};
use super::{Failure, execute};
use crate::ui::style::Style;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const MANIFEST: &str = r#"[package]
name = "shop"
version = "0.1.0"
edition = "2024"

[dependencies]
# The framework facade; keep this comment.
rullst = { version = "13.0.0", default-features = false, features = ["orm", "studio"] } # trailing note
serde = "1.0"
"#;

fn project(manifest: &str, files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Cargo.toml"), manifest).unwrap();
    for (name, contents) in files {
        fs::write(root.path().join(name), contents).unwrap();
    }
    root
}

fn read(root: &Path, file: &str) -> String {
    fs::read_to_string(root.join(file)).unwrap()
}

fn add(root: &Path, name: &str) -> String {
    execute(root, name, false, Style::PLAIN).unwrap()
}

fn features(manifest: &str) -> Vec<String> {
    let parsed: toml::Value = toml::from_str(manifest).unwrap();
    parsed["dependencies"]["rullst"]["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|feature| feature.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn enabling_keeps_comments_and_existing_features_and_is_idempotent() {
    let root = project(MANIFEST, &[(".env", "APP_KEY=local\n")]);
    let first = add(root.path(), "mail");
    let manifest = read(root.path(), "Cargo.toml");
    assert!(manifest.contains("# The framework facade; keep this comment."));
    assert!(manifest.contains("# trailing note"));
    assert!(manifest.contains("serde = \"1.0\""));
    assert_eq!(features(&manifest), ["orm", "studio", "mail"]);
    assert!(first.contains("✓ Cargo.toml"), "{first}");
    assert!(
        first.contains("rullst::mail::Mail::default_sender().await?;"),
        "{first}"
    );
    assert_eq!(
        read(root.path(), ".env"),
        "APP_KEY=local\n",
        ".env is untouched"
    );

    let example = read(root.path(), ".env.example");
    assert!(example.starts_with("# ── Mail (added by `cargo rullst add mail`) ──\n"));
    assert!(example.contains("MAIL_FROM=\"Example App <no-reply@example.com>\"\n"));
    assert!(example.contains("# MAIL_DRIVER=resend\n"));
    assert!(example.contains("# RESEND_API_KEY=mock_resend_key\n"));

    let before = [
        read(root.path(), "Cargo.toml"),
        read(root.path(), ".env.example"),
        read(root.path(), ".env"),
    ];
    let second = add(root.path(), "mail");
    assert!(second.contains("`mail` is already enabled"), "{second}");
    assert!(second.contains("Nothing to change"), "{second}");
    assert!(!second.contains("Wire it in"), "{second}");
    assert_eq!(
        before,
        [
            read(root.path(), "Cargo.toml"),
            read(root.path(), ".env.example"),
            read(root.path(), ".env"),
        ]
    );
}

#[test]
fn version_strings_and_dependency_tables_both_gain_the_feature() {
    let root = project(
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nrullst = \"13.0.0\"\n",
        &[],
    );
    add(root.path(), "auth");
    let manifest = read(root.path(), "Cargo.toml");
    assert!(
        manifest.contains(r#"rullst = { version = "13.0.0", features = ["auth"] }"#),
        "{manifest}"
    );

    let table = "[package]\nname = \"b\"\nversion = \"0.1.0\"\n\n[dependencies.rullst]\n# pinned\nversion = \"13.0.0\"\n";
    let root = project(table, &[]);
    add(root.path(), "ai");
    let manifest = read(root.path(), "Cargo.toml");
    assert!(manifest.contains("# pinned"), "{manifest}");
    let parsed: toml::Value = toml::from_str(&manifest).unwrap();
    assert_eq!(
        parsed["dependencies"]["rullst"]["features"][0].as_str(),
        Some("ai")
    );
}

#[test]
fn a_missing_env_example_is_created_and_existing_entries_are_not_repeated() {
    let root = project(MANIFEST, &[]);
    add(root.path(), "nexus");
    let example = read(root.path(), ".env.example");
    assert!(
        example.contains("NEXUS_ADMIN_USERNAME=\nNEXUS_ADMIN_PASSWORD=\n"),
        "{example}"
    );
    assert!(!root.path().join(".env").exists(), ".env is never created");

    let generated =
        "APP_KEY=REPLACE_WITH_YOUR_32_CHAR_RANDOM_KEY\nMAIL_FROM=\n# MAIL_DRIVER=resend";
    let root = project(MANIFEST, &[(".env.example", generated)]);
    let output = add(root.path(), "mail");
    let example = read(root.path(), ".env.example");
    assert!(example.starts_with(generated), "existing text is kept");
    assert_eq!(example.matches("MAIL_FROM").count(), 1);
    assert_eq!(example.matches("MAIL_DRIVER=").count(), 1);
    assert!(
        example.ends_with("# MAIL_DRIVER=resend\n\n# ── Mail (added by `cargo rullst add mail`) ──\n# RESEND_API_KEY=mock_resend_key\n"),
        "{example:?}"
    );
    assert!(
        output.contains("add RESEND_API_KEY (commented)"),
        "{output}"
    );

    // Auth's only variable is already in every generated .env.example.
    add(root.path(), "auth");
    assert_eq!(
        read(root.path(), ".env.example").matches("APP_KEY").count(),
        1
    );
}

#[test]
fn implied_features_count_as_enabled() {
    let manifest = MANIFEST.replace(r#"["orm", "studio"]"#, r#"["mailer", "auth-sqlite"]"#);
    for name in ["mail", "auth"] {
        let root = project(&manifest, &[(".env.example", "")]);
        let output = add(root.path(), name);
        assert!(output.contains("already enabled"), "{name}: {output}");
        assert_eq!(read(root.path(), "Cargo.toml"), manifest, "{name}");
    }
}

#[test]
fn dry_run_shows_the_diff_and_writes_nothing() {
    let root = project(MANIFEST, &[(".env.example", "APP_KEY=x\n")]);
    let output = execute(root.path(), "ai", true, Style::PLAIN).unwrap();
    assert_eq!(read(root.path(), "Cargo.toml"), MANIFEST);
    assert_eq!(read(root.path(), ".env.example"), "APP_KEY=x\n");
    for expected in [
        "~ Cargo.toml",
        "would enable the `ai` feature of rullst",
        "--- Cargo.toml",
        r#"+ rullst = { version = "13.0.0", default-features = false, features = ["orm", "studio", "ai"] } # trailing note"#,
        "--- .env.example",
        "+ OPENAI_API_KEY=mock_openai_key",
        "rullst::ai::AiClient::auto()?;",
        "Dry run: nothing was written.",
    ] {
        assert!(
            output.contains(expected),
            "missing {expected:?} in\n{output}"
        );
    }
    assert!(!output.contains('\u{1b}'), "plain output has no escapes");
}

#[test]
fn only_rullst_projects_are_accepted() {
    let empty = tempfile::tempdir().unwrap();
    assert_eq!(
        execute(empty.path(), "mail", false, Style::PLAIN),
        Err(Failure::Project)
    );
    let other = project(
        "[package]\nname = \"x\"\n\n[dependencies]\nserde = \"1\"\n",
        &[],
    );
    assert_eq!(
        execute(other.path(), "mail", false, Style::PLAIN),
        Err(Failure::Project)
    );
    assert!(!other.path().join(".env.example").exists());

    let broken = project(
        "[package]\nname = \"x\"\n\n[dependencies\nrullst = \"1\"\n",
        &[],
    );
    let invalid = execute(broken.path(), "mail", false, Style::PLAIN);
    assert!(
        matches!(
            invalid,
            Err(Failure::Add(AddError::InvalidManifest {
                line: Some(4 | 5)
            }))
        ),
        "{invalid:?}"
    );
    let unsupported = project("[dependencies]\nrullst = 13\n", &[]);
    assert_eq!(
        execute(unsupported.path(), "mail", false, Style::PLAIN),
        Err(Failure::Add(AddError::UnsupportedDependency))
    );
    assert_eq!(
        execute(unsupported.path(), "billing", false, Style::PLAIN),
        Err(Failure::Add(AddError::UnknownCapability("billing".into())))
    );
}

#[test]
fn dotenv_gains_only_entries_needed_to_start_and_keeps_line_endings() {
    const NEEDED: Capability = Capability {
        name: "demo",
        summary: "",
        feature: "demo",
        implied_by: &[],
        env_title: "Demo",
        env: &[
            EnvEntry {
                key: "DEMO_URL",
                value: "mock_local",
                commented: false,
                note: &[],
                dev_required: true,
            },
            EnvEntry {
                key: "DEMO_TOKEN",
                value: "",
                commented: false,
                note: &[],
                dev_required: false,
            },
        ],
        snippet: Snippet {
            place: "",
            code: "",
        },
        hints: &[],
    };
    let change = dotenv_change(&NEEDED, "APP_KEY=x\r\n".to_string()).unwrap();
    assert_eq!(
        change.after,
        "APP_KEY=x\r\n\r\n# ── Demo (added by `cargo rullst add demo`) ──\r\nDEMO_URL=mock_local\r\n"
    );
    assert!(change.summary.contains("DEMO_URL"));
    assert!(change.summary.contains("needed to start"));
    // A commented-out value does not configure the application.
    assert!(dotenv_change(&NEEDED, "# DEMO_URL=x\n".to_string()).is_some());
    assert!(dotenv_change(&NEEDED, "export DEMO_URL=real\n".to_string()).is_none());
    for capability in &CAPABILITIES {
        assert!(
            dotenv_change(capability, String::new()).is_none(),
            "{}",
            capability.name
        );
    }

    let example = env_example_change(&NEEDED, Some("A=1\r\n".to_string())).unwrap();
    assert!(
        example.after.ends_with("DEMO_TOKEN=\r\n"),
        "{:?}",
        example.after
    );
    assert!(!example.after.replace("\r\n", "").contains('\n'));
}

#[test]
fn dotenv_keys_are_recognised_with_comments_exports_and_spaces() {
    let text = "A=1\n  # B = 2\nexport C=3\n#comment without key\nD\n=E\nlower_ok=1\n";
    assert_eq!(assigned_keys(text, true), ["A", "B", "C", "lower_ok"]);
    assert_eq!(assigned_keys(text, false), ["A", "C", "lower_ok"]);
}

#[test]
fn the_table_is_consistent_and_never_ships_real_secrets() {
    assert_eq!(
        NAMES.to_vec(),
        CAPABILITIES
            .iter()
            .map(|capability| capability.name)
            .collect::<Vec<_>>()
    );
    for capability in &CAPABILITIES {
        assert!(find(capability.name).is_some());
        assert!(
            (1..=3).contains(&capability.hints.len()),
            "{}",
            capability.name
        );
        assert!(!capability.snippet.code.is_empty() && !capability.snippet.place.is_empty());
        for entry in capability.env {
            let value = entry.value.trim_matches('"');
            assert!(
                value.is_empty()
                    || value.starts_with("mock_")
                    || value.contains("example.com")
                    || [
                        "resend",
                        "127.0.0.1:11434",
                        "REPLACE_WITH_YOUR_32_CHAR_RANDOM_KEY"
                    ]
                    .contains(&value),
                "{}: {} = {}",
                capability.name,
                entry.key,
                entry.value
            );
        }
    }
}

/// Every facade feature that enables a capability's feature, from the
/// workspace's `rullst/Cargo.toml`, matches the table.
#[test]
fn implied_features_match_the_facade_manifest() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rullst/Cargo.toml");
    let Ok(text) = fs::read_to_string(path) else {
        return; // Not in the workspace checkout (for example a packaged crate).
    };
    let parsed: toml::Value = toml::from_str(&text).unwrap();
    let table = parsed["features"].as_table().unwrap();
    let graph: BTreeMap<&str, Vec<&str>> = table
        .iter()
        .map(|(name, enables)| {
            let enables = enables
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|value| value.as_str())
                .collect();
            (name.as_str(), enables)
        })
        .collect();
    let enables = |start: &str, target: &str| {
        let mut stack = vec![start];
        let mut seen = BTreeSet::new();
        while let Some(feature) = stack.pop() {
            if feature == target {
                return true;
            }
            if seen.insert(feature) {
                stack.extend(graph.get(feature).into_iter().flatten().copied());
            }
        }
        false
    };
    for capability in &CAPABILITIES {
        assert!(
            graph.contains_key(capability.feature),
            "{}",
            capability.feature
        );
        let implied: BTreeSet<&str> = graph
            .keys()
            .copied()
            .filter(|feature| {
                *feature != capability.feature && enables(feature, capability.feature)
            })
            .collect();
        let table: BTreeSet<&str> = capability.implied_by.iter().copied().collect();
        assert_eq!(implied, table, "{}", capability.name);
    }
}

#[test]
fn the_report_says_when_dotenv_needs_nothing() {
    let root = project(MANIFEST, &[(".env", "APP_KEY=local\n")]);
    let report = add(root.path(), "mail");
    assert!(
        report.contains(
            "· .env          unchanged: nothing it lacks is needed to start in development"
        ),
        "{report}"
    );
    assert_eq!(read(root.path(), ".env"), "APP_KEY=local\n");
}

#[test]
fn the_command_takes_a_capability_and_a_dry_run_flag() {
    let matches = super::command()
        .try_get_matches_from(["add", "mail", "--dry-run"])
        .unwrap();
    assert_eq!(
        matches.get_one::<String>("capability").map(String::as_str),
        Some("mail")
    );
    assert!(matches.get_flag("dry_run"));
    assert!(
        super::command()
            .try_get_matches_from(["add", "unknown"])
            .is_err()
    );
}

#[test]
fn hints_come_from_the_named_capability() {
    let mail = find("mail").unwrap();
    assert!(!mail.hints.is_empty());
    assert_eq!(super::hints("mail"), mail.hints);
    assert!(super::hints("unknown").is_empty());
}

#[test]
fn running_outside_a_rullst_project_fails() {
    // The unit tests run in the cargo-rullst package, which does not depend
    // on the `rullst` facade; the dry run writes nothing either way.
    let matches = super::command()
        .try_get_matches_from(["add", "mail", "--dry-run"])
        .unwrap();
    assert!(super::run(&matches).is_err());
}

#[test]
fn errors_describe_the_failing_file_or_value() {
    assert_eq!(
        AddError::UnknownCapability("cache".into()).to_string(),
        "`cache` is not a capability"
    );
    assert_eq!(
        AddError::InvalidManifest { line: Some(3) }.to_string(),
        "Cargo.toml is not valid TOML (line 3)"
    );
    assert_eq!(
        AddError::Read {
            file: ".env.example",
            kind: std::io::ErrorKind::PermissionDenied
        }
        .to_string(),
        "could not read .env.example: permission denied"
    );
}

#[test]
fn an_unreadable_env_example_fails_instead_of_counting_as_missing() {
    let root = project(MANIFEST, &[]);
    fs::create_dir(root.path().join(".env.example")).unwrap();
    match execute(root.path(), "mail", true, Style::PLAIN) {
        Err(Failure::Add(AddError::Read { file, .. })) => assert_eq!(file, ".env.example"),
        Err(_) => panic!("unexpected failure"),
        Ok(report) => panic!("a directory is not an empty .env.example: {report}"),
    }
}

#[test]
fn the_manifest_is_written_last() {
    let root = project(MANIFEST, &[]);
    let plan = match super::plan::plan(root.path(), find("mail").unwrap()).unwrap() {
        Ok(plan) => plan,
        Err(_) => panic!("the fixture is a Rullst project"),
    };
    assert!(plan.changes.len() >= 2, "{:?}", plan.changes);
    let written = super::plan::apply(root.path(), &plan).unwrap();
    assert!(
        written.last().unwrap().ends_with("Cargo.toml"),
        "{written:?}"
    );
    assert!(!written[0].ends_with("Cargo.toml"), "{written:?}");
}

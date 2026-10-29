//! Review, application and recovery of a verified project in a private cache.
//!
//! The cache location comes from `XDG_CACHE_HOME`, so every operation runs in
//! a child test process with its own environment.
use super::*;
use serde_json::{Value, json};
use std::fs;

const LOCK: &str = "version = 4\n\n[[package]]\nname = \"rullst\"\nversion = \"13.0.0\"\n";

struct CachedProject {
    _temp: tempfile::TempDir,
    xdg: PathBuf,
    source: PathBuf,
    prepared: PathBuf,
    verified: PathBuf,
}

fn private_directory(path: &Path) {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(path).unwrap();
}

/// A Git project prepared and verified inside `xdg/rullst-update-v1`.
fn cached_project() -> CachedProject {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    fs::set_permissions(&base, fs::Permissions::from_mode(0o700)).unwrap();
    let xdg = base.join("xdg");
    private_directory(&xdg);
    let cache = xdg.join("rullst-update-v1");
    private_directory(&cache);
    let source = base.join("source");
    fs::create_dir_all(source.join("src")).unwrap();
    fs::write(
        source.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = { version = \"13.0.0-alpha.1\", default-features = false }\n",
    )
    .unwrap();
    fs::write(source.join("src/main.rs"), "fn main() {}\n").unwrap();
    assert!(
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&source)
            .status()
            .unwrap()
            .success()
    );
    let paths = snapshot::inventory(&source).unwrap();
    let prepared = cache.join(format!("project-{}", uuid::Uuid::new_v4()));
    private_directory(&prepared);
    super::super::prepare_into(source.clone(), "13.0.0", &paths, &prepared).unwrap();
    let state = State::load(&prepared).unwrap();

    let verified = cache.join(format!("project-{}", uuid::Uuid::new_v4()));
    private_directory(&verified);
    let candidate = verified.join("candidate");
    for input in &state.candidate_records {
        if input.path != "Cargo.lock" && input.sha256.is_some() {
            let target = candidate.join(&input.path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(state.candidate.join(&input.path), target).unwrap();
        }
    }
    fs::write(candidate.join("Cargo.lock"), LOCK).unwrap();
    let files: Vec<_> = state
        .candidate_records
        .iter()
        .map(|input| {
            if input.path == "Cargo.lock" {
                snapshot::record("Cargo.lock", Some(LOCK.as_bytes()))
            } else {
                input.clone()
            }
        })
        .collect();
    let commands: Vec<Value> = super::super::verify::commands(&[], true)
        .iter()
        .enumerate()
        .map(|(index, command)| {
            let mut observation = json!({"program": command.program, "args": command.args,
                "success": true, "stdout": format!("command-{index}.stdout"),
                "stderr": format!("command-{index}.stderr")});
            for stream in ["stdout", "stderr"] {
                let body = format!("{stream} of command {index}");
                fs::write(verified.join(format!("command-{index}.{stream}")), &body).unwrap();
                observation[format!("{stream}_sha256")] =
                    json!(hex::encode(Sha256::digest(body.as_bytes())));
            }
            observation
        })
        .collect();
    let record = json!({
        "schema_version": "rullst.project-verification.v1", "phase": "verified",
        "prepared_directory": prepared, "verified_directory": verified,
        "verified_candidate_directory": candidate, "source": source,
        "target": "13.0.0", "platform": std::env::consts::OS, "features": [],
        "offline": true, "commands": commands, "files": files,
        "execution_authorized_for_this_invocation": true,
        "application_authorized": false, "production_ready": false,
    });
    fs::write(
        verified.join("verification.json"),
        serde_json::to_vec_pretty(&record).unwrap(),
    )
    .unwrap();
    CachedProject {
        _temp: temp,
        xdg,
        source,
        prepared,
        verified,
    }
}

/// Runs `cargo rullst update project <args>` in a child test process.
#[test]
fn evidence_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_EVIDENCE_REPORT") else {
        return;
    };
    let args = std::env::var("RULLST_EVIDENCE_ARGS").unwrap();
    let matches = super::super::command()
        .try_get_matches_from(std::iter::once("project").chain(args.split('\u{1f}')))
        .unwrap();
    let text = if std::env::var_os("RULLST_EVIDENCE_RUN").is_some() {
        match super::super::run(&matches) {
            Ok(()) => "{}".to_string(),
            Err(error) => format!("err {error}"),
        }
    } else {
        match super::super::execute(&matches) {
            Ok(value) => value.to_string(),
            Err(error) => format!("err {error}"),
        }
    };
    fs::write(report, text).unwrap();
}

fn operation(project: &CachedProject, args: &[&str]) -> Result<Value, String> {
    operation_with(project, args, None)
}

fn operation_with(
    project: &CachedProject,
    args: &[&str],
    offline_variable: Option<&str>,
) -> Result<Value, String> {
    let report = project.xdg.parent().unwrap().join("report");
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command.env_remove("CARGO_NET_OFFLINE");
    if let Some(value) = offline_variable {
        command.env("CARGO_NET_OFFLINE", value);
    }
    let status = command
        .args([
            "--exact",
            "update::project::review::tests::evidence_child_fixture",
            "--nocapture",
        ])
        .env("XDG_CACHE_HOME", &project.xdg)
        .env("RULLST_EVIDENCE_REPORT", &report)
        .env("RULLST_EVIDENCE_ARGS", args.join("\u{1f}"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    let text = fs::read_to_string(report).unwrap();
    match text.strip_prefix("err ") {
        Some(error) => Err(error.to_string()),
        None => Ok(serde_json::from_str(&text).unwrap()),
    }
}

fn review(project: &CachedProject) -> Value {
    operation(
        project,
        &["review", "--verified", project.verified.to_str().unwrap()],
    )
    .unwrap()
}

fn apply(project: &CachedProject, action: &str, approval: &str) -> Result<Value, String> {
    operation(
        project,
        &[
            action,
            "--verified",
            project.verified.to_str().unwrap(),
            "--approved-review",
            approval,
        ],
    )
}

#[test]
fn a_reviewed_update_applies_and_recovers_exactly() {
    let project = cached_project();
    let before = fs::read_to_string(project.source.join("Cargo.toml")).unwrap();
    let reviewed = review(&project);
    let digest = reviewed["review_sha256"].as_str().unwrap().to_string();
    assert_eq!(digest.len(), 64);
    assert!(
        reviewed["diff"].as_str().unwrap().contains("+rullst"),
        "{reviewed}"
    );
    assert_eq!(reviewed["review"]["application_authorized"], false);
    assert_eq!(
        reviewed["review"]["changes"].as_array().unwrap().len(),
        2,
        "{reviewed}"
    );

    assert_eq!(
        apply(&project, "apply", &"0".repeat(64)).unwrap_err(),
        "project preparation rejected: approved review differs from the current evidence/diff; review again"
    );
    assert_eq!(
        apply(&project, "recover", &digest).unwrap_err(),
        "project preparation rejected: no application intent exists for this verification"
    );
    fs::write(project.verified.join("application.json"), "{}").unwrap();
    assert_eq!(
        apply(&project, "apply", &digest).unwrap_err(),
        "project preparation rejected: this operation already has an application intent; recover it or prepare a new update"
    );
    fs::remove_file(project.verified.join("application.json")).unwrap();
    let applied = apply(&project, "apply", &digest).unwrap();
    assert_eq!(applied["phase"], "applied");
    assert_eq!(applied["file_operations"], 2);
    let after = fs::read_to_string(project.source.join("Cargo.toml")).unwrap();
    assert_ne!(after, before);
    assert!(after.contains("13.0.0"));
    assert_eq!(
        fs::read_to_string(project.source.join("Cargo.lock")).unwrap(),
        LOCK
    );
    // The applied source no longer matches the preparation.
    assert!(apply(&project, "apply", &digest).is_err());

    let restored = apply(&project, "recover", &digest).unwrap();
    assert_eq!(restored["phase"], "restored");
    assert_eq!(restored["file_operations"], 2);
    assert_eq!(
        fs::read_to_string(project.source.join("Cargo.toml")).unwrap(),
        before
    );
    assert!(!project.source.join("Cargo.lock").exists());
    let intent: Value =
        serde_json::from_slice(&fs::read(project.verified.join("application.json")).unwrap())
            .unwrap();
    assert_eq!(intent["phase"], "restored");
}

#[test]
fn recovery_refuses_a_divergent_user_edit() {
    let project = cached_project();
    let digest = review(&project)["review_sha256"]
        .as_str()
        .unwrap()
        .to_string();
    apply(&project, "apply", &digest).unwrap();
    fs::write(project.source.join("Cargo.toml"), "edited after apply").unwrap();
    assert_eq!(
        apply(&project, "recover", &digest).unwrap_err(),
        "project preparation rejected: recovery refuses a divergent user edit; no files were restored"
    );
    assert_eq!(
        fs::read_to_string(project.source.join("Cargo.toml")).unwrap(),
        "edited after apply"
    );
}

#[test]
fn a_changed_source_or_record_blocks_review_and_application() {
    let project = cached_project();
    let digest = review(&project)["review_sha256"]
        .as_str()
        .unwrap()
        .to_string();
    fs::write(
        project.source.join("src/main.rs"),
        "fn main() { edited() }\n",
    )
    .unwrap();
    assert!(
        operation(
            &project,
            &["review", "--verified", project.verified.to_str().unwrap()]
        )
        .is_err()
    );
    assert!(apply(&project, "apply", &digest).is_err());
    fs::write(project.source.join("src/main.rs"), "fn main() {}\n").unwrap();
    let mut record: Value =
        serde_json::from_slice(&fs::read(project.verified.join("verification.json")).unwrap())
            .unwrap();
    record["offline"] = json!(false);
    fs::write(
        project.verified.join("verification.json"),
        serde_json::to_vec_pretty(&record).unwrap(),
    )
    .unwrap();
    assert!(apply(&project, "apply", &digest).is_err());
    assert_eq!(
        fs::read_to_string(project.source.join("src/main.rs")).unwrap(),
        "fn main() {}\n"
    );
}

fn prepared(project: &CachedProject) -> &str {
    project.prepared.to_str().unwrap()
}

#[test]
fn a_verification_plan_reports_its_offline_policy() {
    let project = cached_project();
    let plan = operation(
        &project,
        &["verify", "--prepared", prepared(&project), "--dry-run"],
    )
    .unwrap();
    assert_eq!(plan["offline"], true);
    assert_eq!(plan["execution_authorized"], false);
    assert_eq!(
        plan["commands"][2]["args"],
        json!(["update", "--workspace", "--offline"])
    );
    let online = operation(
        &project,
        &[
            "verify",
            "--prepared",
            prepared(&project),
            "--dry-run",
            "--allow-network",
        ],
    )
    .unwrap();
    assert_eq!(online["offline"], false);
    // An offline environment only conflicts with an explicit network request.
    let offline_environment = operation_with(
        &project,
        &["verify", "--prepared", prepared(&project), "--dry-run"],
        Some("true"),
    )
    .unwrap();
    assert_eq!(offline_environment["offline"], true);
    assert_eq!(
        operation_with(
            &project,
            &[
                "verify",
                "--prepared",
                prepared(&project),
                "--dry-run",
                "--allow-network"
            ],
            Some("true"),
        )
        .unwrap_err(),
        "project preparation rejected: --allow-network conflicts with CARGO_NET_OFFLINE"
    );
}

#[test]
fn verification_stops_at_the_first_failing_cargo_command() {
    let project = cached_project();
    // Offline resolution cannot find the unpublished framework release, so
    // the third command (`cargo update --workspace --offline`) fails.
    let error = operation(
        &project,
        &[
            "verify",
            "--prepared",
            prepared(&project),
            "--allow-project-code",
        ],
    )
    .unwrap_err();
    assert!(
        error.ends_with("command-2.stderr; no acceptance or application was recorded"),
        "{error}"
    );
}

/// `cargo rullst update project <args>` through `run`, with its output.
fn run_in_child(project: &CachedProject, args: &[&str]) -> (String, String, String) {
    let report = project.xdg.parent().unwrap().join("run-report");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "update::project::review::tests::evidence_child_fixture",
            "--nocapture",
        ])
        .env("XDG_CACHE_HOME", &project.xdg)
        .env("RULLST_EVIDENCE_REPORT", &report)
        .env("RULLST_EVIDENCE_ARGS", args.join("\u{1f}"))
        .env("RULLST_EVIDENCE_RUN", "1")
        .env_remove("CARGO_NET_OFFLINE")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success());
    (
        fs::read_to_string(report).unwrap(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn verification_prints_its_plan_and_progress() {
    let project = cached_project();
    let (result, stdout, _) = run_in_child(
        &project,
        &["verify", "--prepared", prepared(&project), "--dry-run"],
    );
    assert_eq!(result, "{}");
    assert!(
        stdout.contains("\"schema_version\": \"rullst.project-verification-plan.v1\""),
        "{stdout}"
    );
    let (result, _, stderr) = run_in_child(
        &project,
        &[
            "verify",
            "--prepared",
            prepared(&project),
            "--allow-project-code",
        ],
    );
    assert!(result.starts_with("err "), "{result}");
    assert!(
        stderr.contains("Verifying candidate: rustc --version --verbose"),
        "{stderr}"
    );
}

#[test]
fn review_prints_the_diff_and_its_digest() {
    let project = cached_project();
    let verified = project.verified.to_str().unwrap();
    let digest = review(&project)["review_sha256"]
        .as_str()
        .unwrap()
        .to_string();
    let (result, stdout, _) = run_in_child(&project, &["review", "--verified", verified]);
    assert_eq!(result, "{}");
    assert!(stdout.contains("+rullst"), "{stdout}");
    assert!(
        stdout.contains(&format!("Review SHA-256: \"{digest}\"")),
        "{stdout}"
    );
    assert!(stdout.contains("Application is not authorized by this review."));
    let (_, stdout, _) = run_in_child(&project, &["review", "--verified", verified, "--json"]);
    assert!(
        stdout.contains(&format!("\"review_sha256\": \"{digest}\"")),
        "{stdout}"
    );
}

#[path = "evidence_intent_tests.rs"]
mod intent;

/// Loads the evidence named by `RULLST_REVALIDATE_VERIFIED`, then changes
/// the mode of `RULLST_REVALIDATE_FILE` and reports both revalidations, one
/// per line; does nothing unless a parent test names a report file.
#[test]
fn revalidation_child_fixture() {
    use std::os::unix::fs::PermissionsExt;
    let Some(report) = std::env::var_os("RULLST_REVALIDATE_REPORT") else {
        return;
    };
    let verified = PathBuf::from(std::env::var_os("RULLST_REVALIDATE_VERIFIED").unwrap());
    let changed = PathBuf::from(std::env::var_os("RULLST_REVALIDATE_FILE").unwrap());
    let evidence = Evidence::load(&verified).unwrap();
    let mode = fs::metadata(&changed).unwrap().permissions().mode() & 0o777;
    let mode = if mode == 0o600 { 0o640 } else { 0o600 };
    fs::set_permissions(&changed, fs::Permissions::from_mode(mode)).unwrap();
    let outcome = [
        evidence.revalidate(),
        evidence.revalidate_ignoring(&std::collections::BTreeSet::new()),
    ]
    .map(|result| result.map_or_else(|error| error.to_string(), |()| "ok".to_string()))
    .join("\n");
    fs::write(report, outcome).unwrap();
}

/// The review digest covers file modes when the evidence is loaded; a mode
/// changed after that is refused by the revalidation before and after staging.
#[test]
fn revalidation_refuses_a_mode_changed_after_loading() {
    let project = cached_project();
    let report = project.xdg.parent().unwrap().join("revalidation");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "update::project::review::tests::revalidation_child_fixture",
            "--nocapture",
        ])
        .env("XDG_CACHE_HOME", &project.xdg)
        .env("RULLST_REVALIDATE_REPORT", &report)
        .env("RULLST_REVALIDATE_VERIFIED", &project.verified)
        .env("RULLST_REVALIDATE_FILE", project.source.join("Cargo.toml"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    let refused = "project preparation rejected: source permissions changed after review";
    assert_eq!(
        fs::read_to_string(report).unwrap(),
        format!("{refused}\n{refused}")
    );
}

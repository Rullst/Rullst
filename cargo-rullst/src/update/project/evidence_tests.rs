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
    let text = match super::super::execute(&matches) {
        Ok(value) => value.to_string(),
        Err(error) => format!("err {error}"),
    };
    fs::write(report, text).unwrap();
}

fn operation(project: &CachedProject, args: &[&str]) -> Result<Value, String> {
    let report = project.xdg.parent().unwrap().join("report");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
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

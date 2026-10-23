//! Project operations dispatch to their checked commands and refuse early.
use super::*;

const BUILD: &str =
    "project preparation rejected: build metadata and oversized versions are unsupported";
const UNSUPPORTED: &str = "project preparation rejected: unsupported project operation";

fn project(args: &[&str]) -> ArgMatches {
    command()
        .try_get_matches_from(std::iter::once("project").chain(args.iter().copied()))
        .unwrap()
}

fn rejection(args: &[&str]) -> String {
    match execute(&project(args)) {
        Ok(_) => panic!("the project operation was accepted"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn project_errors_debug_print_their_message() {
    assert_eq!(
        format!("{:?}", ProjectError::Invalid("fixture")),
        "project preparation rejected: fixture"
    );
}

#[test]
fn each_operation_reaches_its_own_checks() {
    let empty = tempfile::tempdir().unwrap();
    let empty = empty.path().to_str().unwrap();
    assert_eq!(
        rejection(&["prepare", "--project", empty, "--to", "13.0.0+build"]),
        BUILD
    );
    assert_eq!(
        rejection(&["verify", "--prepared", "/rullst-missing-preparation"]),
        "project preparation rejected: review the preparation first; --allow-project-code is required because Cargo builds and tests execute project code"
    );
    assert_ne!(
        rejection(&["review", "--verified", "/rullst-missing-verification"]),
        UNSUPPORTED
    );
    let approval =
        "project preparation rejected: approval must be the complete lowercase SHA-256 from review";
    for action in ["apply", "recover"] {
        let args = [action, "--verified", empty, "--approved-review", "nope"];
        assert_eq!(rejection(&args), approval, "{action}");
        assert!(run(&project(&args)).is_err());
    }
}

#[test]
fn prepared_targets_are_plain_versions_of_at_most_64_characters() {
    let empty = tempfile::tempdir().unwrap();
    let empty = empty.path().to_str().unwrap();
    let longest = format!("1.0.0-{}", "a".repeat(58));
    assert_eq!(longest.len(), 64);
    // Accepted versions continue to the project inventory, which an empty
    // directory fails.
    for accepted in ["13.0.0", longest.as_str()] {
        assert_ne!(
            rejection(&["prepare", "--project", empty, "--to", accepted]),
            BUILD,
            "{accepted}"
        );
    }
    assert_eq!(
        rejection(&[
            "prepare",
            "--project",
            empty,
            "--to",
            &format!("{longest}a")
        ]),
        BUILD
    );
}

#[test]
fn verification_features_follow_the_verify_policy() {
    let matches = verify::command()
        .try_get_matches_from(["verify", "--prepared", ".", "--features", "serde"])
        .unwrap();
    assert_eq!(
        verification_features(&matches).unwrap(),
        ["--features", "serde"]
    );
}

/// A Git project prepared into a private stage directory, without the cache.
pub(super) struct PreparedProject {
    _temp: tempfile::TempDir,
    pub source: PathBuf,
    pub stage: PathBuf,
}

pub(super) fn prepared_project() -> PreparedProject {
    prepared_project_with("rullst = { version = \"13.0.0-alpha.1\", default-features = false }\n")
}

/// A prepared project whose manifest declares `dependencies`.
pub(super) fn prepared_project_with(dependencies: &str) -> PreparedProject {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source = base.join("source");
    std::fs::create_dir_all(source.join("src")).unwrap();
    std::fs::write(
        source.join("Cargo.toml"),
        format!(
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n{dependencies}"
        ),
    )
    .unwrap();
    std::fs::write(source.join("src/main.rs"), "fn main() {}\n").unwrap();
    assert!(
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&source)
            .status()
            .unwrap()
            .success()
    );
    let paths = snapshot::inventory(&source).unwrap();
    let stage = base.join("stage");
    std::fs::create_dir(&stage).unwrap();
    prepare_into(source.clone(), "13.0.0", &paths, &stage).unwrap();
    PreparedProject {
        _temp: temp,
        source,
        stage,
    }
}

#[test]
fn preparation_copies_and_plans_the_project_into_its_stage() {
    let project = prepared_project();
    let prepared: serde_json::Value =
        serde_json::from_slice(&std::fs::read(project.stage.join("preparation.json")).unwrap())
            .unwrap();
    assert_eq!(prepared["schema_version"], "rullst.project-preparation.v1");
    assert_eq!(prepared["target"], "13.0.0");
    assert_eq!(
        prepared["source"],
        serde_json::json!(project.source.clone())
    );
    assert_eq!(
        std::fs::read_to_string(project.stage.join("before/src/main.rs")).unwrap(),
        "fn main() {}\n"
    );
    assert!(
        std::fs::read_to_string(project.stage.join("candidate/Cargo.toml"))
            .unwrap()
            .contains("13.0.0")
    );
}

use super::*;
use crate::ai::backend::Backend;
use crate::ai::input::Input;
use crate::ai::mock::MockAssistant;
use crate::ai::session::{Mode, Session, Settings};
use crate::ai::term::Style;
use crate::generators::build::AssistFinding;
use rullst_ai::{AiGuardrails, StreamingAiClient};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const MAIN: &str = "use rullst::htmx::{HtmxRequest, render_page};\n\nasync fn home(htmx: HtmxRequest) -> Html<String> {\n    render_page(&htmx, \"Home\", body())\n}\n";

fn finding(code: &'static str, must_change: bool, path: &str, line: usize) -> AssistFinding {
    AssistFinding {
        code,
        label: if must_change { "MUST-CHANGE" } else { "REVIEW" },
        must_change,
        path: path.to_string(),
        line,
        message: "a message",
        row: "Starter page language",
        guidance: "Call `render_page_with_lang` with the page's language.",
    }
}

fn plan(findings: Vec<AssistFinding>) -> AssistPlan {
    AssistPlan {
        target: "13.0.0-alpha.1".to_string(),
        catalog: "rullst-upgrade-rules-v4",
        pending_changes: 1,
        summary:
            "Project: demo\nSource findings (rullst-upgrade-rules-v4): 0 must-change, 1 review"
                .to_string(),
        findings,
    }
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn project() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(directory.path()).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), MAIN).unwrap();
    fs::write(
        root.join(".env"),
        "SECRET=1\nOTEL_EXPORTER_OTLP_ENDPOINT=x\n",
    )
    .unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["add", "Cargo.toml", "src/main.rs"]);
    git(&root, &["commit", "-q", "-m", "initial"]);
    (directory, root)
}

fn findings() -> Vec<AssistFinding> {
    vec![
        finding("V13-OTLP-ENVIRONMENT", false, ".env", 2),
        finding("V13-RENDER-PAGE-LANGUAGE", false, "src/main.rs", 4),
        finding("V13-HTML-DYNAMIC-EVENT-HANDLER", true, "src/main.rs", 1),
    ]
}

#[test]
fn the_brief_orders_findings_and_shares_only_allowed_files() {
    let (_guard, root) = project();
    let brief = brief(&root, &plan(findings()));
    let block = &brief.attachments[0];
    let must = block.find("V13-HTML-DYNAMIC-EVENT-HANDLER").unwrap();
    let review = block.find("V13-RENDER-PAGE-LANGUAGE").unwrap();
    assert!(must < review, "must-change findings come first");
    assert!(block.contains("line: render_page(&htmx, \"Home\", body())"));
    assert!(!block.contains("SECRET"), "protected files are never read");
    assert_eq!(brief.attachments.len(), 2, "one excerpt for src/main.rs");
    assert!(brief.attachments[1].contains("source=\"file_src/main.rs\""));
    assert!(brief.notes[0].contains(".env"));
    // Only the referenced rows reach the trusted instructions.
    assert!(brief.instructions.starts_with(UPGRADE_HEADING));
    assert_eq!(
        brief.instructions.matches("Starter page language").count(),
        3
    );
    assert!(!brief.instructions.contains("Kubernetes"));
    assert!(brief.instructions.contains("--keep-on-failure"));
    assert!(
        AiGuardrails::inspect(&brief.instructions)
            .threat()
            .is_none()
    );
}

#[test]
fn the_offline_fix_inserts_the_language_after_the_first_argument() {
    assert_eq!(
        crate::ai::mock::with_language("render_page(&htmx, \"Home\", body())"),
        Some("rullst::htmx::render_page_with_lang(&htmx, \"en\", \"Home\", body())".to_string())
    );
    assert_eq!(
        crate::ai::mock::with_language(
            "Ok(rullst::htmx::render_page(&request(\"a, b\"), &format!(\"{x}\"), body))"
        ),
        Some(
            "Ok(rullst::htmx::render_page_with_lang(&request(\"a, b\"), \"en\", &format!(\"{x}\"), body))"
                .to_string()
        )
    );
    assert_eq!(crate::ai::mock::with_language("render_page(x)"), None);
    assert_eq!(
        crate::ai::mock::with_language("render_page_with_lang(a, b)"),
        None
    );
}

async fn run(root: &Path, mode: Mode, answers: &[&str]) -> String {
    let backend = Backend::Mock(StreamingAiClient::new(MockAssistant));
    let settings = Settings {
        label: "offline mock".to_string(),
        notice: None,
        style: Style { color: false },
        mode,
        root: Some(root.to_path_buf()),
        cwd: root.to_path_buf(),
        prices: None,
    };
    let mut session = Session::new(&backend, settings, Vec::new(), Input::script(answers));
    let plan = plan(findings());
    let brief = brief(root, &plan);
    session.upgrade(&plan.summary, brief).await;
    String::from_utf8(session.into_output()).unwrap()
}

#[tokio::test]
async fn the_offline_assistant_fixes_one_finding_after_review_and_a_checkpoint() {
    let (_guard, root) = project();
    // Apply the edit, decline the proposed `cargo check` and the final offer.
    let output = run(&root, Mode::Execute, &["y", "n", "n"]).await;
    let main = fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(
        main.contains("rullst::htmx::render_page_with_lang(&htmx, \"en\", \"Home\", body())"),
        "{output}"
    );
    assert!(output.contains("Source findings (rullst-upgrade-rules-v4)"));
    assert!(
        output.contains("Checkpoint refs/rullst/ai-checkpoints/"),
        "{output}"
    );
    assert_eq!(
        output.matches("Apply?").count(),
        2,
        "the edit and `cargo check`: {output}"
    );
    assert!(output.contains("Run `cargo check` now?"), "{output}");
    assert!(output.contains("the reviewed upgrade step ran"), "{output}");
    let refs = git(&root, &["for-each-ref", "refs/rullst/ai-checkpoints"]);
    assert_eq!(refs.lines().count(), 1);
}

#[tokio::test]
async fn plan_only_upgrade_sessions_never_execute() {
    let (_guard, root) = project();
    let output = run(
        &root,
        Mode::PlanOnly("not an interactive terminal"),
        &["y", "y"],
    )
    .await;
    assert!(output.contains("Plan only"), "{output}");
    assert!(
        output.contains("render_page_with_lang"),
        "the diff is shown: {output}"
    );
    assert!(output.contains("(not executed)"));
    assert_eq!(fs::read_to_string(root.join("src/main.rs")).unwrap(), MAIN);
    assert_eq!(git(&root, &["for-each-ref", "refs/rullst"]), "");
}

#[tokio::test]
async fn without_a_supported_finding_the_offline_assistant_only_explains() {
    let (_guard, root) = project();
    let backend = Backend::Mock(StreamingAiClient::new(MockAssistant));
    let settings = Settings {
        label: "offline mock".to_string(),
        notice: None,
        style: Style { color: false },
        mode: Mode::Execute,
        root: Some(root.clone()),
        cwd: root.clone(),
        prices: None,
    };
    let mut session = Session::new(&backend, settings, Vec::new(), Input::script(&[]));
    let plan = plan(vec![finding("V13-MODEL-ALL", false, "src/main.rs", 4)]);
    session.upgrade(&plan.summary, brief(&root, &plan)).await;
    let output = String::from_utf8(session.into_output()).unwrap();
    assert!(
        output.contains("only rewrites `render_page` calls"),
        "{output}"
    );
    assert!(!output.contains("Apply?"));
    assert_eq!(fs::read_to_string(root.join("src/main.rs")).unwrap(), MAIN);
}

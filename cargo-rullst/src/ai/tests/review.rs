use super::diff::{Collected, FileDiff, Scope, annotate, collect, section_paths, sections};
use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Secret-shaped value built at run time, so no literal token is committed.
fn aws_key() -> String {
    format!("{}{}", "AKIA", "Q7W3E9R2T5Y8U1I4")
}

/// A repository with one commit, then: a staged edit of `src/staged.rs`, an
/// unstaged edit of `src/unstaged.rs`, a tracked `.env` edit, an untracked
/// file and an ignored one.
fn repository() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(directory.path()).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/staged.rs"), "fn a() {}\n").unwrap();
    fs::write(root.join("src/unstaged.rs"), "fn b() {}\n").unwrap();
    fs::write(root.join(".env"), "PORT=3000\n").unwrap();
    fs::write(root.join(".gitignore"), "ignored.rs\n").unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-q", "-m", "init", "--no-gpg-sign"]);
    fs::write(
        root.join("src/staged.rs"),
        "fn a() {\n    let value: u32 = \"1\".parse().unwrap();\n}\n",
    )
    .unwrap();
    git(&root, &["add", "src/staged.rs"]);
    fs::write(
        root.join("src/unstaged.rs"),
        format!("fn b() {{}}\nconst KEY: &str = \"{}\";\n", aws_key()),
    )
    .unwrap();
    fs::write(root.join(".env"), "PORT=3000\nSTRIPE_SECRET_KEY=value\n").unwrap();
    fs::write(root.join("src/untracked.rs"), "fn c() { panic!(\"x\"); }\n").unwrap();
    fs::write(root.join("ignored.rs"), "fn d() {}\n").unwrap();
    (directory, root)
}

fn paths(collected: &Collected) -> Vec<&str> {
    collected
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect()
}

fn git_available() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

#[test]
fn the_scope_selects_the_diff_and_untracked_files_need_the_flag() {
    if !git_available() {
        return;
    }
    let (_guard, root) = repository();
    let program = OsStr::new("git");
    assert_eq!(diff::work_tree(program, &root.join("src")).unwrap(), root);

    let working = collect(program, &root, &Scope::WorkingTree, false).unwrap();
    assert_eq!(paths(&working), ["src/staged.rs", "src/unstaged.rs"]);
    assert_eq!(working.omitted.len(), 1);
    assert_eq!(working.omitted[0].path, ".env");
    assert!(working.omitted[0].reason.contains("protected"));
    assert_eq!(working.redactions, 1);

    let staged = collect(program, &root, &Scope::Staged, false).unwrap();
    assert_eq!(paths(&staged), ["src/staged.rs"]);

    let untracked = collect(program, &root, &Scope::WorkingTree, true).unwrap();
    assert_eq!(
        paths(&untracked),
        ["src/staged.rs", "src/unstaged.rs", "src/untracked.rs"]
    );
    assert!(!untracked.files.iter().any(|file| file.path == "ignored.rs"));

    git(&root, &["add", "src/unstaged.rs"]);
    git(&root, &["commit", "-q", "-m", "second", "--no-gpg-sign"]);
    let first = git(&root, &["rev-list", "--max-parents=0", "HEAD"]);
    let base = collect(
        program,
        &root,
        &Scope::Base(first.trim().to_string()),
        false,
    )
    .unwrap();
    assert_eq!(paths(&base), ["src/staged.rs", "src/unstaged.rs"]);
    assert!(collect(program, &root, &Scope::Base("--output=x".into()), false).is_err());
    assert!(collect(program, &root, &Scope::Base("no-such-branch".into()), false).is_err());

    let outside = tempfile::tempdir().unwrap();
    if let Err(error) = diff::work_tree(program, outside.path()) {
        assert!(error.contains("git work tree"), "{error}");
    }
}

#[test]
fn secrets_never_reach_the_provider_payload() {
    if !git_available() {
        return;
    }
    let (_guard, root) = repository();
    let mut collected = collect(OsStr::new("git"), &root, &Scope::WorkingTree, true).unwrap();
    let payload = payload(&mut collected);
    let sent: String = payload
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert!(!sent.contains(&aws_key()));
    assert!(!sent.contains("STRIPE_SECRET_KEY"), ".env is omitted");
    assert!(sent.contains("[redacted: AWS access key ID]"));
    assert!(sent.contains("<untrusted-data source=\"diff/src/unstaged.rs\">"));
    assert!(sent.contains("+    2| const KEY"), "{sent}");
    assert_eq!(payload.reviewed.len(), 3);
}

#[test]
fn diffs_are_split_numbered_and_bounded() {
    let diff = "diff --git a/src/a.rs b/src/a.rs\nindex 1..2 100644\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,3 +10,4 @@ fn x()\n context\n-old\n+new\n+more\n tail\ndiff --git a/key.pem b/key.pem\nBinary files a/key.pem and b/key.pem differ\ndiff --git a/old.rs b/new.rs\nsimilarity index 100%\nrename from old.rs\nrename to new.rs\n";
    let parts = sections(diff);
    assert_eq!(parts.len(), 3);
    assert_eq!(section_paths(parts[0]), ["src/a.rs"]);
    assert_eq!(section_paths(parts[1]), ["key.pem"]);
    assert_eq!(section_paths(parts[2]), ["new.rs", "old.rs"]);
    let mut redactions = 0;
    let annotated = annotate(parts[0], &mut redactions).unwrap();
    assert!(annotated.contains("+++ b/src/a.rs\n@@ -1,3 +10,4 @@ fn x()\n    10|"));
    assert!(annotated.contains("-     | old\n+   11| new\n+   12| more\n    13| tail\n"));

    // A large file is truncated; past the total bound files are omitted.
    let big = "+    1| x\n".repeat(4_000);
    let mut collected = Collected::default();
    for index in 0..6 {
        collected.files.push(FileDiff {
            path: format!("src/f{index}.rs"),
            text: big.clone(),
        });
    }
    let payload = payload(&mut collected);
    assert_eq!(payload.reviewed.len(), 3);
    assert_eq!(payload.truncated.len(), 3);
    assert_eq!(payload.truncated[0].bytes, big.len());
    assert!(payload.messages[1].content.contains("truncated"));
    assert_eq!(collected.omitted.len(), 3);
    assert!(collected.omitted[0].reason.contains("96 KiB"));
}

#[test]
fn the_offline_review_is_deterministic_and_parses() {
    let message = "<untrusted-data source=\"diff/src/a.rs\">\n@@ -1 +1,3 @@\n+    1| let x = y.unwrap();\n+    2| let key = \"[redacted: AWS access key ID]\";\n    3| ok\n</untrusted-data>\n\n<untrusted-data source=\"diff/tests/a.rs\">\n+    4| y.unwrap();\n</untrusted-data>\n\nReview this change set.";
    let answer = mock_review(message);
    assert_eq!(answer, mock_review(message));
    let parsed = report::parse(&answer);
    assert_eq!(parsed.format_error, None);
    assert_eq!(parsed.ignored_actions, 0);
    let found: Vec<(String, Option<u64>, report::Severity)> = parsed
        .findings
        .iter()
        .map(|finding| (finding.file.clone(), finding.line, finding.severity))
        .collect();
    assert_eq!(
        found,
        [
            ("src/a.rs".to_string(), Some(2), report::Severity::High),
            ("src/a.rs".to_string(), Some(1), report::Severity::Medium),
        ]
    );
}

#[test]
fn action_blocks_are_ignored_and_free_text_is_reported() {
    let answer = "Sure.\n```rullst-action\n{\"action\": \"write_file\", \"path\": \"a.rs\", \"content\": \"x\"}\n```\n```rullst-review\n{\"summary\": \"ok\", \"findings\": [{\"file\": \"a.rs\", \"line\": 0, \"severity\": \"CRITICAL\", \"title\": \"t\"}, {\"title\": \"\"}]}\n```\n";
    let parsed = report::parse(answer);
    assert_eq!(parsed.ignored_actions, 1);
    assert_eq!(parsed.findings.len(), 1);
    assert_eq!(parsed.findings[0].line, None);
    assert_eq!(parsed.findings[0].severity, report::Severity::High);
    let free = report::parse("Looks fine to me.");
    assert!(free.format_error.is_some() && free.findings.is_empty());
}

#[test]
fn the_json_report_has_a_stable_schema() {
    let report = Report {
        schema: report::SCHEMA,
        provider: "offline mock".to_string(),
        offline: true,
        scope: ScopeReport {
            diff: "working-tree",
            base: None,
            include_untracked: false,
        },
        files: Files {
            reviewed: vec!["src/a.rs".to_string()],
            omitted: Vec::new(),
            truncated: Vec::new(),
            output_cut: false,
        },
        redactions: 0,
        findings: report::parse(&mock_review(
            "<untrusted-data source=\"diff/src/a.rs\">\n+    1| a.unwrap();\n",
        ))
        .findings,
        summary: Some("s".to_string()),
        ignored_actions: 0,
        format_error: None,
        raw_answer: None,
        usage: UsageReport::default(),
    };
    let value = serde_json::to_value(&report).unwrap();
    let keys = |value: &serde_json::Value| {
        let mut keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    };
    assert_eq!(value["schema"], "rullst.ai-review.v1");
    assert_eq!(
        keys(&value),
        [
            "files",
            "findings",
            "format_error",
            "ignored_actions",
            "offline",
            "provider",
            "raw_answer",
            "redactions",
            "schema",
            "scope",
            "summary",
            "usage"
        ]
    );
    assert_eq!(
        keys(&value["findings"][0]),
        ["detail", "file", "line", "severity", "suggestion", "title"]
    );
    assert_eq!(value["findings"][0]["severity"], "medium");
    assert_eq!(
        keys(&value["files"]),
        ["omitted", "output_cut", "reviewed", "truncated"]
    );
    assert_eq!(
        keys(&value["usage"]),
        ["input_tokens", "line", "output_tokens", "total_tokens"]
    );
}

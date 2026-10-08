#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use std::fs;

/// A temporary project with `files` (relative path, contents).
pub(super) fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temporary project");
    for (path, contents) in files {
        let path = directory.path().join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("fixture directory");
        fs::write(path, contents).expect("fixture file");
    }
    directory
}

fn missing_tools() -> Tools<'static> {
    Tools {
        cargo: OsStr::new("rullst-missing-cargo-fixture"),
        git: OsStr::new("rullst-missing-git-fixture"),
    }
}

/// A Stripe-shaped live key built at run time, so no literal key is committed.
fn stripe_key() -> String {
    ["sk", "live", "Q1w2E3r4T5y6U7i8O9p0"].join("_")
}

const VULNERABLE_APP: &str = r#"
fn routes() -> Router {
    routes![
        post("/<script>alert(1)</script>" => store),
        post("/login" => login),
        get("/users/:id" => show),
    ]
}
fn cookie() -> &'static str { "app_session=abc; Path=/" }
"#;

const PROTECTED_APP: &str = r#"
fn main() { let _ = Server::new(router()).rate_limit(RateLimiter::new(config)); }
fn routes() -> Router {
    routes![
        post("/login" => login),
        // rullst-access: public — published posts are intentionally public.
        get("/posts/:id" => show),
    ]
}
fn cookie() -> String { format!("app_session={}; Path=/; HttpOnly; SameSite=Lax{}", value, secure) }
fn page() -> Html { html! { <html lang="en"><img src="/a.png" alt="Logo" /></html> } }
"#;

fn by_id<'a>(report: &'a Report, id: &str) -> &'a Check {
    report
        .checks
        .iter()
        .find(|check| check.spec.id == id)
        .unwrap_or_else(|| panic!("check {id}"))
}

#[test]
fn every_check_reports_findings_for_a_vulnerable_fixture_and_none_for_a_protected_one() {
    let vulnerable = project(&[
        ("src/main.rs", VULNERABLE_APP),
        (
            "templates/page.html",
            "<html><img src=\"x.png\"><input name=\"q\"></html>",
        ),
    ]);
    let report = collect(vulnerable.path(), &[], &missing_tools());
    for id in [
        "security_headers",
        "csrf",
        "cookies",
        "auth_rate_limit",
        "idor",
        "a11y_img_alt",
        "a11y_form_labels",
        "a11y_html_lang",
    ] {
        assert!(
            matches!(by_id(&report, id).status, EvidenceStatus::Findings(_)),
            "{id}: {:?}",
            by_id(&report, id).status
        );
    }
    assert!(report.fails());

    let protected = project(&[("src/main.rs", PROTECTED_APP)]);
    let report = collect(protected.path(), &[], &missing_tools());
    for id in [
        "security_headers",
        "csrf",
        "cookies",
        "auth_rate_limit",
        "idor",
        "a11y_img_alt",
        "a11y_html_lang",
    ] {
        assert_eq!(
            by_id(&report, id).status,
            EvidenceStatus::NoFindings,
            "{id}"
        );
    }
    // Unavailable tools are NOT CHECKED, which never fails the audit.
    assert!(matches!(
        by_id(&report, "committed_secrets").status,
        EvidenceStatus::NotChecked(_)
    ));
    assert!(matches!(
        by_id(&report, "dependency_audit").status,
        EvidenceStatus::NotChecked(_)
    ));
    assert!(!report.fails(), "{:?}", report.checks);
}

#[test]
fn only_the_last_error_line_of_a_tool_is_reported() {
    assert_eq!(
        last_error_line(
            "    Fetching advisory database from `/home/me/db`\nerror: not found: Couldn't load Cargo.lock\nCaused by: x\n"
        ),
        "error: not found: Couldn't load Cargo.lock"
    );
    assert_eq!(last_error_line("first\nlast\n"), "last");
    assert_eq!(last_error_line(""), "no output");
}

#[test]
fn idor_findings_keep_their_location() {
    let finding = idor_finding("File 'src/main.rs:4': parameterized route `/users/:id` missing");
    assert_eq!(finding.location(), "src/main.rs:4");
    assert_eq!(finding.message, "parameterized route `/users/:id` missing");
    assert_eq!(idor_finding("other text").location(), "");
}

#[test]
fn a_project_without_sources_is_not_checked_instead_of_clean() {
    let empty = project(&[]);
    let report = collect(empty.path(), &[], &missing_tools());
    assert!(
        report
            .checks
            .iter()
            .all(|check| matches!(check.status, EvidenceStatus::NotChecked(_))),
        "{:?}",
        report.checks
    );
    assert!(!report.fails());
}

#[cfg(unix)]
fn fake_cargo(directory: &Path, audit_exit: u8) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let path = directory.join(format!("cargo-{audit_exit}"));
    fs::write(
        &path,
        format!(
            "#!/bin/sh\nif [ \"$2\" = --version ]; then exit 0; fi\nif [ {audit_exit} -ne 0 ]; then echo 'error: 1 vulnerability found!' >&2; fi\nexit {audit_exit}\n"
        ),
    )
    .expect("fake cargo");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("executable");
    path
}

#[cfg(unix)]
#[test]
fn dependency_check_reuses_cargo_audit_and_its_exceptions() {
    let tools = tempfile::tempdir().expect("tool directory");
    let root = project(&[]);
    let clean = fake_cargo(tools.path(), 0);
    let vulnerable = fake_cargo(tools.path(), 1);

    let check = dependency_check(root.path(), clean.as_os_str(), &[]);
    assert_eq!(check.status, EvidenceStatus::NoFindings);
    let governed = ["RUSTSEC-2099-0001".to_string()];
    let check = dependency_check(root.path(), clean.as_os_str(), &governed);
    assert_eq!(
        check.status,
        EvidenceStatus::NoFindingsOutsideExceptions(governed.to_vec())
    );
    assert!(check.detail.contains("remain unresolved"));
    let check = dependency_check(root.path(), vulnerable.as_os_str(), &[]);
    assert!(matches!(check.status, EvidenceStatus::Error(_)));
    assert!(
        check
            .detail
            .ends_with("Last error: error: 1 vulnerability found!")
    );
    assert!(check.status.fails());
    let check = dependency_check(root.path(), OsStr::new("rullst-missing-cargo"), &[]);
    assert!(matches!(check.status, EvidenceStatus::NotChecked(_)));
}

fn rendered(report: &Report) -> [String; 3] {
    [
        render(report, ReportFormat::Markdown).unwrap(),
        render(report, ReportFormat::Html).unwrap(),
        render(report, ReportFormat::Json).unwrap(),
    ]
}

#[test]
fn no_format_contains_the_secret_or_a_pass_or_certification_claim() {
    let secret = stripe_key();
    let root = project(&[
        ("src/main.rs", VULNERABLE_APP),
        ("config.toml", &format!("stripe = \"{secret}\"\n")),
    ]);
    let mut report = collect(root.path(), &[], &missing_tools());
    let secrets = secrets::check(root.path(), Ok(vec![PathBuf::from("config.toml")]));
    assert_eq!(secrets.status, EvidenceStatus::Findings(1));
    report.checks[4] = secrets;
    for output in rendered(&report) {
        assert!(!output.contains(&secret), "a format printed the secret");
        assert!(output.contains("sk_l…"));
        assert!(!output.contains("PASS"));
        assert!(!output.to_ascii_lowercase().contains("compliant"));
        assert!(!output.to_ascii_lowercase().contains("certified"));
        assert!(output.contains("penetration test"));
        assert!(output.contains(catalog::ASVS_SOURCE));
        assert!(!output.contains('\u{1b}'), "no ANSI sequence");
    }
}

#[test]
#[cfg_attr(windows, ignore = "Windows file names cannot contain <, > or \"")]
fn html_is_self_contained_and_escapes_hostile_names_and_values() {
    let hostile = "x\"><script>alert(1)</script>.html";
    let root = project(&[
        ("src/main.rs", VULNERABLE_APP),
        (
            &format!("templates/{hostile}"),
            "<html lang=\"en\"><img src=\"a\"></html>",
        ),
    ]);
    let report = collect(root.path(), &[], &missing_tools());
    assert!(
        by_id(&report, "a11y_img_alt")
            .findings
            .iter()
            .any(|finding| finding.file.contains("<script>"))
    );
    let html = render(&report, ReportFormat::Html).unwrap();
    assert!(
        !html.contains("<script"),
        "no script and no unescaped value"
    );
    assert!(html.contains("x&quot;&gt;&lt;script&gt;alert(1)&lt;/script&gt;.html"));
    assert!(html.contains("POST /&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(html.contains("prefers-color-scheme: dark"));
    for external in ["<link", " src=", "@import", "url("] {
        assert!(!html.contains(external), "external reference {external}");
    }
    let markdown = render(&report, ReportFormat::Markdown).unwrap();
    assert!(!markdown.contains("<script>"));
    assert!(markdown.contains("&lt;script&gt;"));
}

/// Sorted, so the snapshot holds with or without serde_json's `preserve_order`.
fn keys(value: &serde_json::Value) -> Vec<String> {
    let mut keys: Vec<String> = value.as_object().expect("object").keys().cloned().collect();
    keys.sort();
    keys
}

#[test]
fn json_schema_keys_are_stable() {
    let root = project(&[
        ("src/main.rs", VULNERABLE_APP),
        (
            "src/models.rs",
            "#[derive(Orm)]\nstruct User { #[privacy] name: String, email: String }\n",
        ),
    ]);
    let report = collect(root.path(), &[], &missing_tools());
    let document: serde_json::Value =
        serde_json::from_str(&render(&report, ReportFormat::Json).unwrap()).unwrap();
    assert_eq!(document["schema_version"], "rullst.cli-audit-report.v1");
    assert_eq!(
        keys(&document),
        [
            "accessibility_standard",
            "checks",
            "exit_non_zero",
            "generated_at",
            "not_evaluated",
            "notice",
            "personal_data",
            "schema_version",
            "standard",
            "tool"
        ]
    );
    assert_eq!(
        keys(&document["standard"]),
        [
            "level",
            "level1_mapped",
            "level1_requirements",
            "mapping_note",
            "name",
            "source",
            "version"
        ]
    );
    assert_eq!(document["standard"]["version"], "5.0.0");
    assert_eq!(document["standard"]["level1_requirements"], 70);
    assert_eq!(document["standard"]["level1_mapped"], 7);
    let check = &document["checks"][0];
    assert_eq!(
        keys(check),
        [
            "asvs",
            "asvs_chapter",
            "count",
            "detail",
            "docs",
            "exceptions",
            "findings",
            "fix",
            "group",
            "id",
            "status",
            "title",
            "wcag"
        ]
    );
    assert_eq!(keys(&check["asvs"][0]), ["id", "level", "section"]);
    assert_eq!(
        keys(&check["findings"][0]),
        ["file", "line", "message", "preview"]
    );
    let ids: Vec<_> = document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| check["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        ids,
        [
            "security_headers",
            "csrf",
            "cookies",
            "auth_rate_limit",
            "committed_secrets",
            "dependency_audit",
            "idor",
            "a11y_img_alt",
            "a11y_form_labels",
            "a11y_html_lang"
        ]
    );
    let personal = &document["personal_data"];
    assert_eq!(
        keys(personal),
        [
            "asvs_chapter",
            "count",
            "detail",
            "docs",
            "fields",
            "review",
            "status"
        ]
    );
    assert_eq!(
        keys(&personal["fields"][0]),
        [
            "classification",
            "encrypted",
            "field",
            "file",
            "line",
            "model"
        ]
    );
    assert_eq!(personal["review"], 1);
    let not_evaluated = &document["not_evaluated"][0];
    assert_eq!(
        keys(not_evaluated),
        ["chapter", "name", "requirements", "status", "whole_chapter"]
    );
    assert_eq!(not_evaluated["status"], "not_evaluated");
    assert_eq!(document["exit_non_zero"], true);
}

#[test]
fn level_one_catalog_matches_the_official_count_and_reports_unmapped_chapters() {
    let total: usize = catalog::LEVEL1.iter().map(|(_, _, ids)| ids.len()).sum();
    assert_eq!(total, 70);
    let root = project(&[]);
    let report = collect(root.path(), &[], &missing_tools());
    let open = catalog::not_evaluated(&report);
    let v1 = open.iter().find(|chapter| chapter.chapter == "V1").unwrap();
    assert!(v1.whole_chapter);
    let v3 = open.iter().find(|chapter| chapter.chapter == "V3").unwrap();
    assert!(!v3.whole_chapter);
    assert!(!v3.requirements.contains(&"V3.4.1"));
    assert!(v3.requirements.contains(&"V3.2.1"));
    assert_eq!(catalog::level1_coverage(&report), (7, 70));
}

#[test]
fn the_terminal_summary_is_grouped_and_plain_without_colour() {
    let root = project(&[("src/main.rs", VULNERABLE_APP)]);
    let report = collect(root.path(), &[], &missing_tools());
    let text = summary::render(&report, "SECURITY_REPORT.md", Style::PLAIN);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains("Security checks\n"));
    assert!(text.contains("Accessibility checks\n"));
    assert!(text.contains("  ✗ CSRF — 2 finding(s)"), "{text}");
    assert!(
        text.contains("  ! Committed secrets — not checked:"),
        "{text}"
    );
    assert!(text.contains("docs: https://rullst.github.io/Rullst/book/security-report.html#csrf"));
    assert!(text.contains("Report written to SECURITY_REPORT.md"));
    assert!(text.contains("exit status 1"));
}

#[cfg(unix)]
#[test]
fn the_report_is_never_written_through_a_symlink() {
    let directory = tempfile::tempdir().unwrap();
    let victim = directory.path().join("victim");
    fs::write(&victim, "keep").unwrap();
    let output = directory.path().join("SECURITY_REPORT.md");
    std::os::unix::fs::symlink(&victim, &output).unwrap();
    assert!(write_output(&output, b"report", true).is_err());
    assert_eq!(fs::read_to_string(victim).unwrap(), "keep");
}

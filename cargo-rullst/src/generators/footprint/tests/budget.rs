//! `--check-budget` and `--budget FILE`: parsing, checking, rendering and
//! the exit status, end to end in a child test process.
use super::super::budget::{self, Budget, BudgetError, Source};
use super::run::footprint_with;
use super::*;

fn budget(text: &str) -> Budget {
    budget::parse(text, "Rullst.toml").unwrap()
}

fn names(budgets: &budget::Budgets) -> Vec<(&str, &str)> {
    budgets
        .limits
        .iter()
        .map(|limit| (limit.name, limit.status))
        .collect()
}

#[test]
fn a_budget_lists_known_limits_in_report_order() {
    let parsed = budget(
        "[app]\nname = \"shop\"\n\n[footprint.budgets]\npage_requests = 30\ncpu_ms_per_request = 2.5\nbinary_size_bytes = 20_000_000\n",
    );
    let checked = budget::check(&parsed, &sample(true, None));
    assert_eq!(
        names(&checked),
        [
            ("binary_size_bytes", "passed"),
            ("cpu_ms_per_request", "exceeded"),
            ("page_requests", "passed"),
        ]
    );
    assert_eq!(checked.limits[1].allowed, 2.5);
    assert_eq!(checked.limits[1].measured, Some(3.75));
    assert_eq!(checked.limits[1].unit, "milliseconds");
    assert_eq!(checked.limits[0].unit, "bytes");
    assert_eq!(checked.limits[2].unit, "count");
    assert_eq!(checked.source.as_deref(), Some("Rullst.toml"));
}

#[test]
fn unusable_budgets_are_refused_without_echoing_values() {
    let error = |text: &str| budget::parse(text, "budget.toml").unwrap_err();
    assert_eq!(
        error("[app]\nname = 1\n"),
        BudgetError::Missing("budget.toml".into())
    );
    assert_eq!(
        error("[footprint]\nbudgets = 3\n"),
        BudgetError::Missing("budget.toml".into())
    );
    assert_eq!(
        error("[footprint.budgets]\n"),
        BudgetError::Empty("budget.toml".into())
    );
    let unknown = error("[footprint.budgets]\npage_weight = 1\n").to_string();
    assert!(
        unknown.contains("`page_weight` is not a limit"),
        "{unknown}"
    );
    assert!(unknown.contains("page_first_visit_bytes"), "{unknown}");
    for value in ["-1", "\"500 KiB\"", "nan", "inf", "true"] {
        assert_eq!(
            error(&format!("[footprint.budgets]\nidle_rss_bytes = {value}\n")),
            BudgetError::Invalid {
                file: "budget.toml".into(),
                key: "idle_rss_bytes".into()
            },
            "{value}"
        );
    }
    let syntax = error("[footprint.budgets]\nidle_rss_bytes = secret-value\n").to_string();
    assert!(
        syntax.starts_with("budget.toml is not valid TOML at "),
        "{syntax}"
    );
    assert!(!syntax.contains("secret-value"), "{syntax}");
    assert_eq!(
        budget::parse("[footprint.budgets]\nidle_rss_bytes = 0\n", "x")
            .map(|parsed| budget::check(&parsed, &sample(false, None)).limits[0].status),
        Ok("exceeded")
    );
}

#[test]
fn budgets_are_read_from_rullst_toml_or_a_file() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    assert!(matches!(
        budget::load(&Source::RullstToml, root),
        Err(BudgetError::Read(name, _)) if name == "Rullst.toml"
    ));
    std::fs::write(
        root.join("Rullst.toml"),
        "[footprint.budgets]\ndependency_count = 400\n",
    )
    .unwrap();
    std::fs::write(
        root.join("ci-budget.toml"),
        "[footprint.budgets]\ndependency_count = 100\n",
    )
    .unwrap();
    let report = sample(false, None);
    let project_limit = budget::load(&Source::RullstToml, root).unwrap();
    assert_eq!(budget::check(&project_limit, &report).status, "passed");
    let file = Source::File("ci-budget.toml".into());
    let file_limit = budget::load(&file, root).unwrap();
    let checked = budget::check(&file_limit, &report);
    assert_eq!(checked.status, "exceeded");
    assert_eq!(checked.source.as_deref(), Some("ci-budget.toml"));
    std::fs::write(root.join("big.toml"), "#".repeat(1024 * 1024 + 1)).unwrap();
    assert_eq!(
        budget::load(&Source::File("big.toml".into()), root),
        Err(BudgetError::TooLarge("big.toml".into()))
    );
}

#[test]
fn unmeasured_figures_are_skipped_never_passed() {
    let every = budget(
        "[footprint.budgets]\nbinary_size_bytes = 1e9\nimage_size_bytes = 1e9\nidle_rss_bytes = 1e9\ndependency_count = 1000\ncpu_ms_per_request = 100\nbytes_per_request = 1e6\npage_first_visit_bytes = 1e9\npage_repeat_visit_bytes = 1e9\npage_requests = 100\npage_third_party_origins = 5\n",
    );
    let quiet = budget::check(&every, &sample(false, None));
    assert_eq!(
        names(&quiet),
        [
            ("binary_size_bytes", "passed"),
            ("image_size_bytes", "skipped"),
            ("idle_rss_bytes", "passed"),
            ("dependency_count", "passed"),
            ("cpu_ms_per_request", "skipped"),
            ("bytes_per_request", "skipped"),
            ("page_first_visit_bytes", "skipped"),
            ("page_repeat_visit_bytes", "skipped"),
            ("page_requests", "skipped"),
            ("page_third_party_origins", "skipped"),
        ]
    );
    assert_eq!((quiet.passed, quiet.exceeded, quiet.skipped), (3, 0, 7));
    assert_eq!(quiet.status, "passed");
    assert_eq!(quiet.limits[1].measured, None);
    assert_eq!(quiet.limits[1].reason.as_deref(), Some("no image given"));
    assert_eq!(
        quiet.limits[4].reason.as_deref(),
        Some("no benchmark ran; pass --bench")
    );
    assert_eq!(
        quiet.limits[9].reason.as_deref(),
        Some("no page was weighed; pass --page-weight")
    );

    let measured = budget::check(&every, &sample(true, None));
    assert_eq!(measured.skipped, 1, "{measured:?}");
    assert_eq!(measured.limits[9].measured, Some(1.0));
    assert_eq!(measured.limits[6].measured, Some(48_000.0));
    assert_eq!(measured.limits[7].measured, Some(1_200.0));
    assert_eq!(measured.limits[8].measured, Some(6.0));
    assert_eq!(measured.limits[5].measured, Some(1_500.0));

    let none = budget::check(
        &budget("[footprint.budgets]\nimage_size_bytes = 1\n"),
        &sample(true, None),
    );
    assert_eq!((none.status, none.skipped), ("skipped", 1));
    assert_eq!(
        budget::figure("not_a_limit", &sample(true, None)),
        Err("not a limit".into())
    );
}

#[test]
fn the_budget_section_names_each_limit_with_measured_and_allowed() {
    let plain = Style { color: false };
    let mut report = sample(true, None);
    assert!(!render::render(&report, plain).contains("Budgets"));
    report.budgets = budget::check(
        &budget(
            "[footprint.budgets]\ncpu_ms_per_request = 2\nidle_rss_bytes = 64_000_000\nimage_size_bytes = 1\n",
        ),
        &report,
    );
    let text = render::render(&report, plain);
    for expected in [
        "Budgets (Rullst.toml [footprint.budgets])",
        "  passed    idle_rss_bytes            16.0 MiB of 61.0 MiB allowed",
        "  EXCEEDED  cpu_ms_per_request        3.750 ms > 2.000 ms allowed",
        "  SKIPPED   image_size_bytes          not measured: no image given",
        "1 passed, 1 exceeded, 1 skipped (a skipped limit was not measured, so it did not pass)",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }
    let error = budget::exceeded_report(&report.budgets);
    assert_eq!(error.to_string(), "1 footprint limit(s) exceeded");
    let error: &(dyn std::error::Error + 'static) = error.as_ref();
    assert_eq!(crate::ui::error_report::report(error, &[]), 1);
}

#[test]
fn the_budget_flags_choose_the_file() {
    let parse = |arguments: &[&str]| {
        options::command()
            .try_get_matches_from(std::iter::once("footprint").chain(arguments.iter().copied()))
            .map(|matches| options::Options::from_matches(&matches).budget)
            .unwrap()
    };
    assert_eq!(parse(&[]), None);
    assert_eq!(parse(&["--check-budget"]), Some(Source::RullstToml));
    assert_eq!(
        parse(&["--budget", "ci.toml"]),
        Some(Source::File("ci.toml".into()))
    );
    assert_eq!(
        parse(&["--check-budget", "--budget", "ci.toml"]),
        Some(Source::File("ci.toml".into()))
    );
}

/// An exceeded limit fails the run after the report; a passing budget does
/// not; an unusable budget stops it before anything is measured.
#[test]
fn an_exceeded_budget_fails_the_run_after_the_report() {
    let files = [(
        "Rullst.toml",
        "[footprint.budgets]\ndependency_count = 0\nimage_size_bytes = 1\n",
    )];
    let (_, outcome, shown, stderr) =
        footprint_with(&files, &[], None, &["--check-budget", "--json"]);
    assert_eq!(outcome, "1 footprint limit(s) exceeded");
    let report: serde_json::Value = serde_json::from_str(&shown).unwrap();
    assert_eq!(report["budgets"]["status"], "exceeded");
    assert_eq!(report["budgets"]["limits"][0]["name"], "image_size_bytes");
    assert_eq!(report["budgets"]["limits"][0]["status"], "skipped");
    assert_eq!(report["budgets"]["limits"][1]["name"], "dependency_count");
    assert_eq!(report["budgets"]["limits"][1]["measured"], 1.0);
    assert_eq!(report["budgets"]["limits"][1]["status"], "exceeded");
    assert!(stderr.contains("Footprint budget exceeded"), "{stderr}");
    assert!(
        stderr.contains("dependency_count: 1 > 0 allowed"),
        "{stderr}"
    );

    let files = [("Rullst.toml", "[footprint.budgets]\ndependency_count = 5\n")];
    let (_, outcome, shown, _) = footprint_with(&files, &[], None, &["--check-budget"]);
    assert_eq!(outcome, "ok");
    assert!(
        shown.contains("  passed    dependency_count          1 of 5 allowed"),
        "{shown}"
    );

    let (_, outcome, shown, _) = footprint_with(&[], &[], None, &["--check-budget"]);
    assert_eq!(outcome, "Rullst.toml could not be read (entity not found)");
    assert!(shown.is_empty(), "{shown}");
}

//! A build or start-up failure: the bounded build log, the fix chosen from
//! the actual error, and the report that still carries the static figures.
use super::super::failure::{Failure, build_excerpt, start_fix};
use super::*;

#[test]
fn a_failed_build_shows_its_first_error_without_progress_lines() {
    let mut log = String::from("Cargo build failed.\n");
    for index in 0..200 {
        log.push_str(&format!("   Compiling dependency-{index} v1.0.0\n"));
    }
    log.push_str("warning: unused variable: `x`\n  --> src/lib.rs:1:5\n\n");
    log.push_str("error[E0425]: cannot find value `y` in this scope\n --> src/main.rs:3:13\n");
    for index in 0..30 {
        log.push_str(&format!("  | context line {index}\n"));
    }
    log.push_str("    Building [=====>   ] 120/300\n");
    let excerpt = build_excerpt(&log);
    assert!(
        excerpt.starts_with("error[E0425]: cannot find value `y` in this scope\n --> src/main.rs:3:13\n  | context line 0"),
        "{excerpt}"
    );
    assert!(!excerpt.contains("Compiling"), "{excerpt}");
    assert!(!excerpt.contains("warning: unused"), "{excerpt}");
    assert!(!excerpt.contains("Building ["), "{excerpt}");
    assert_eq!(excerpt.lines().count(), 21, "{excerpt}");
    assert!(
        excerpt.ends_with("\n… 12 more line(s); run `cargo build --release` for the whole log"),
        "{excerpt}"
    );

    // Without an `error` line the excerpt starts at the top; a short log is
    // shown whole; long lines stop at the character budget.
    assert_eq!(
        build_excerpt("Cargo produced no application executable; check the bin target"),
        "Cargo produced no application executable; check the bin target"
    );
    assert_eq!(
        build_excerpt("error: a\n   Compiling b v1\nerror: c\n"),
        "error: a\nerror: c"
    );
    let long = format!("error: {}\n{}\n", "x".repeat(1500), "y".repeat(600));
    let excerpt = build_excerpt(&long);
    assert!(excerpt.starts_with("error: xxx"), "{excerpt}");
    assert!(!excerpt.contains('y'), "{excerpt}");
    assert!(excerpt.ends_with("\n… 1 more line(s); run `cargo build --release` for the whole log"));
    // A word that only starts like a progress verb is kept.
    assert_eq!(
        build_excerpt("Compilingx\nRunning\n"),
        "Compilingx\nRunning"
    );
    let failure = Failure::build(&log);
    assert_eq!(failure.stage, "build");
    assert_eq!(failure.message, build_excerpt(&log));
}

#[test]
fn the_start_up_fix_follows_the_actual_error() {
    let database = "The app could not use its database";
    for (environment, happened) in [
        (
            "production",
            "the app exited during start-up (exit status: 1).\nLast stderr lines:\nError: pool timed out while waiting for an open connection",
        ),
        ("staging", "Error: Database(\"no such table: users\")"),
        (
            "production",
            "error returned from database: password authentication failed",
        ),
        ("production", "sqlx::Error: Configuration"),
        ("production", "connecting to Postgres failed"),
        ("production", "MySQL server has gone away"),
        ("production", "unable to open SQLite file"),
    ] {
        assert!(
            start_fix(environment, happened).starts_with(database),
            "{happened}"
        );
    }
    assert!(start_fix("production", "APP_KEY is the development key").starts_with("Set APP_KEY"));
    assert!(
        start_fix("production", "BILLING_PROVIDER is not configured")
            .starts_with("Production needs real billing credentials")
    );
    assert!(
        start_fix("staging", "Stripe secret key is missing")
            .starts_with("Production needs real billing credentials")
    );
    assert!(!start_fix("production", "the app exited").contains("billing"));

    let failure = Failure::start(
        "the app exited during start-up (exit status: 1).".to_string(),
        "Error: pool timed out while waiting for an open connection",
        "production",
    );
    assert_eq!(
        failure.message,
        "the app exited during start-up (exit status: 1).\nLast stderr lines:\nError: pool timed out while waiting for an open connection"
    );
    assert!(failure.fix.starts_with(database), "{}", failure.fix);
    assert_eq!(failure.unmeasured(), "the app did not start");
    let quiet = Failure::start("the app did not answer".to_string(), "", "staging");
    assert_eq!(quiet.message, "the app did not answer");
    assert!(
        quiet
            .fix
            .starts_with("Run the release binary with RULLST_ENV=staging")
    );
    assert_eq!(
        Failure::build("error: x").unmeasured(),
        "the release build failed"
    );
    assert_eq!(
        Failure::reach("http://127.0.0.1:1/").unmeasured(),
        "the app was not reachable"
    );
}

/// A failed build: no URL or process, the static figures and the failure.
fn failed_build() -> Report {
    let mut report = sample(false, None);
    let failure = Failure::build("error: cannot find value `y`");
    report.target.url = None;
    report.target.pid = None;
    report.target.executable = None;
    report.target.process_lookup = failure.unmeasured().to_string();
    report.measured.binary_size_bytes = Metric::not_measured("bytes", failure.unmeasured());
    report.measured.idle_rss_bytes = Metric::not_measured("bytes", failure.unmeasured());
    report.benchmark = Benchmark::not_run(failure.unmeasured());
    report.failure = Some(failure);
    report
}

#[test]
fn a_failed_start_keeps_the_report_shape_with_the_failure_inside() {
    let value = serde_json::to_value(failed_build()).unwrap();
    assert_eq!(
        value["failure"],
        serde_json::json!({
            "stage": "build",
            "title": "Release build failed",
            "message": "error: cannot find value `y`",
            "fix": "Fix the build errors (`cargo build --release`), then run footprint again.",
        })
    );
    assert_eq!(value["target"]["url"], serde_json::Value::Null);
    assert_eq!(value["measured"]["dependency_count"]["value"], 312);
    assert_eq!(
        value["benchmark"]["cpu_time_seconds"]["reason"],
        "the release build failed"
    );

    let text = render::render(&failed_build(), Style { color: false });
    for expected in [
        "URL           NONE",
        "Process       NOT RUNNING",
        "Dependencies  312",
        "Benchmark     NOT RUN",
        "Not measured\n  Release build failed: the release build failed\n  Figures that need the running app are NOT MEASURED; the error and its fix follow on stderr.\n\nNotes",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }
    let measured = render::render(&sample(false, None), Style { color: false });
    assert!(!measured.contains("Not measured"), "{measured}");
    assert!(measured.contains("Process       PID 4242"), "{measured}");
}

#[test]
fn the_reported_executable_is_the_file_name() {
    let binary = std::path::Path::new("target").join("release").join("app");
    assert_eq!(file_name(&binary).as_deref(), Some("app"));
    assert_eq!(file_name(std::path::Path::new("/")), None);
}

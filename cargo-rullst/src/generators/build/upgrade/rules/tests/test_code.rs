//! The rules about production behaviour skip test code: inline
//! `#[cfg(test)]` modules, test functions and other `#[cfg(test)]` items of a
//! production file, and test files. Every other rule still reports them.

use super::super::test_code::{self, TestLines};
use super::*;

const COOKIE: &str = "V13-CSRF-COOKIE-IN-PAGE";
const LAYERS: &str = "V13-ROUTER-BASELINE-LAYERS";
const STATIC: &str = "V13-CSRF-STATIC-PREFIXES";
const BOUNDS: &str = "V13-PAGE-BOUNDS";
const PROFILE: &str = "V13-PORTFOLIO-PROFILE";
const VERSION: &str = "V13-HEALTH-FRAMEWORK-VERSION";
const FIELD: &str = "V13-HEALTH-PROBE-VERSION-FIELD";

fn found(file: &str, source: &str) -> Vec<(&'static str, usize)> {
    scan_with(source, file, &WorkspaceFacts::default())
}

/// A controller that reads the raw CSRF cookie on line 3 and whose inline
/// test module (lines 6 to 13) reads it again.
const CONTROLLER: &str = r#"use rullst::server::HeaderMap;
pub(crate) fn token(cookies: &str) -> Option<&str> {
    cookies.strip_prefix("rullst_csrf=")
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_cookie_is_read() {
        let cookie = "rullst_csrf=abc".strip_prefix("rullst_csrf=");
        assert_eq!(cookie, Some("abc"));
    }
}
"#;

#[test]
fn an_inline_test_module_of_a_production_file_is_skipped() {
    assert_eq!(
        found("src/controllers/auth_controller.rs", CONTROLLER),
        [(COOKIE, 3)]
    );
    // Without the production helper, only the test module is left.
    let tests_only = CONTROLLER.replace(
        "    cookies.strip_prefix(\"rullst_csrf=\")",
        "    Some(cookies)",
    );
    assert_eq!(found("src/controllers/auth_controller.rs", &tests_only), []);
    // A whole test file is test code too.
    assert_eq!(
        found("src/controllers/auth_controller_tests.rs", CONTROLLER),
        []
    );
}

#[test]
fn test_functions_and_cfg_test_items_are_test_code_but_cfg_not_test_is_not() {
    let read = "\"c\".strip_prefix(\"rullst_csrf=\")";
    for skipped in [
        format!("#[test]\nfn t() {{ {read}; }}\n"),
        format!("#[tokio::test]\nasync fn t() {{ {read}; }}\n"),
        format!("#[actix_rt::test]\nasync fn t() {{ {read}; }}\n"),
        format!("#[cfg(test)]\nfn helper() -> Option<&'static str> {{ {read} }}\n"),
        format!("#[cfg(test)]\nimpl App {{ fn t(&self) {{ {read}; }} }}\n"),
        format!("impl App {{\n    #[cfg(test)]\n    fn t(&self) {{ {read}; }}\n}}\n"),
        format!("fn outer() {{\n    #[cfg(test)]\n    fn t() {{ {read}; }}\n}}\n"),
        format!("#[cfg(test)]\nmod a {{ mod b {{ fn t() {{ {read}; }} }} }}\n"),
    ] {
        assert_eq!(found("src/lib.rs", &skipped), [], "{skipped}");
    }
    for (reported, line) in [
        (
            format!("#[cfg(not(test))]\nfn helper() -> Option<&'static str> {{ {read} }}\n"),
            2,
        ),
        (
            format!(
                "#[cfg(feature = \"test\")]\nfn helper() -> Option<&'static str> {{ {read} }}\n"
            ),
            2,
        ),
        (
            format!("#[inline]\nfn helper() -> Option<&'static str> {{ {read} }}\n"),
            2,
        ),
        (
            format!("impl App {{\n    #[inline]\n    fn t(&self) {{ {read}; }}\n}}\n"),
            3,
        ),
    ] {
        assert_eq!(
            found("src/lib.rs", &reported),
            [(COOKIE, line)],
            "{reported}"
        );
    }
}

#[test]
fn the_other_production_rules_skip_test_modules_too() {
    let in_tests = |body: &str| format!("#[cfg(test)]\nmod tests {{\n{body}\n}}\n");
    for (code, body) in [
        (
            LAYERS,
            "fn r(router: Router) -> Router { router.layer(axum::middleware::from_fn(csrf_middleware)) }",
        ),
        (
            STATIC,
            "use tower_http::services::ServeDir;\nfn r() -> Router { Router::new().nest_service(\"/files\", ServeDir::new(\"a\")) }",
        ),
        (
            BOUNDS,
            "async fn a(page: u64) { Doc::query().paginate(page, 20).await; }",
        ),
        (PROFILE, "async fn a() { let _ = Profile::find(1).await; }"),
    ] {
        assert_eq!(
            found("src/lib.rs", &format!("{body}\n")),
            [(code, 1 + usize::from(code == STATIC))],
            "{code} outside a test module"
        );
        assert_eq!(found("src/lib.rs", &in_tests(body)), [], "{code}");
    }
}

#[test]
fn rules_that_tests_also_observe_still_report_test_code() {
    // A test that reads the probe's removed `version` key fails in v13.
    let probe = "#[cfg(test)]\nmod tests {\n    #[tokio::test]\n    async fn probe() {\n        let body = json(get(\"/health\")).await;\n        assert_eq!(body[\"version\"], \"13.0.0-alpha.5\");\n    }\n}\n";
    assert_eq!(found("src/main.rs", probe), [(VERSION, 6)]);
    // A struct literal of a type that lost a field no longer compiles.
    let literal = "#[cfg(test)]\nmod tests {\n    fn probe() -> HealthProbeResponse {\n        HealthProbeResponse { status: \"UP\", uptime_seconds: 1, version: \"1\" }\n    }\n}\n";
    assert_eq!(found("src/main.rs", literal), [(FIELD, 4)]);
    // The ORM sandbox attribute's own rule reports it; the cookie read in
    // the test does not.
    let sandbox = "#[rullst_orm::test]\nasync fn t() { \"c\".strip_prefix(\"rullst_csrf=\"); }\n";
    assert_eq!(found("src/lib.rs", sandbox), [("V13-ORM-SANDBOX-TEST", 1)]);
}

#[test]
fn test_lines_run_from_the_attribute_to_the_end_of_the_item() {
    let file = syn::parse_file(CONTROLLER).unwrap();
    let test = TestLines::of(&file, Path::new("src/controllers/auth_controller.rs"));
    let covered: Vec<usize> = (1..=14).filter(|line| test.covers(*line)).collect();
    assert_eq!(covered, (6..=13).collect::<Vec<_>>());
    let whole = TestLines::of(&file, Path::new("tests/auth.rs"));
    assert!((1..=14).all(|line| whole.covers(line)));
    let none = TestLines::of(
        &syn::parse_file("fn main() {}\n").unwrap(),
        Path::new("src/main.rs"),
    );
    assert_eq!(none, TestLines::default());
    assert!(!none.covers(1));
}

#[test]
fn test_files_are_named_as_cargo_and_rullst_name_them() {
    for path in [
        "tests/security.rs",
        "tests/common/mod.rs",
        "src/tests.rs",
        "src/auth_tests.rs",
        "src/auth_test.rs",
        "src/controllers/tests/csrf.rs",
    ] {
        assert!(test_code::is_test_path(Path::new(path)), "{path}");
    }
    for path in [
        "src/main.rs",
        "src/testsuite.rs",
        "src/latest.rs",
        "src/tests_helpers.rs",
        "benches/tests.rs.bak",
    ] {
        assert!(!test_code::is_test_path(Path::new(path)), "{path}");
    }
}

#[test]
fn exactly_the_rules_about_production_behaviour_skip_test_code() {
    let skipping: Vec<&str> = CATALOG
        .iter()
        .filter(|rule| test_code::production_only(rule))
        .map(|rule| rule.code)
        .collect();
    assert_eq!(
        skipping,
        [
            "V13-ERP-DASHBOARD-ACCESS",
            "V13-LMS-RECORD-PROGRESS",
            "V13-CREDENTIAL-RATE-LIMIT",
            "V13-PORTFOLIO-PROFILE",
            "V13-PAGE-CSP-NONCE",
            "V13-ERP-STORE-ORDER",
            "V13-PAGE-BOUNDS",
            "V13-CSRF-STATIC-PREFIXES",
            "V13-CSRF-COOKIE-IN-PAGE",
            "V13-ROUTER-BASELINE-LAYERS",
            "V13-AUTH-SESSION-REGISTRY",
            "V13-AUTH-SESSION-EPOCH-CALLS",
        ]
    );
    // Each is a review rule: code that no longer compiles is never skipped.
    for rule in test_code::PRODUCTION_ONLY {
        assert_eq!(rule.kind, FindingKind::Review, "{}", rule.code);
    }
}

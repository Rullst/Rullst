#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::generators::audit_report::sources::routes;
use crate::generators::audit_report::tests::project;

fn load(files: &[(&str, &str)]) -> (tempfile::TempDir, ProjectSources, Config) {
    let root = project(files);
    let sources = ProjectSources::load(root.path());
    let config = SecurityConfig::load(root.path());
    (root, sources, config)
}

fn messages(check: &Check) -> Vec<String> {
    check
        .findings
        .iter()
        .map(|finding| format!("{} {}", finding.location(), finding.message))
        .collect()
}

#[test]
fn routes_are_recognized_in_routes_macros_and_axum_calls() {
    let found = routes(
        "post(\"/posts\" => store), get(\"/\" => index)\n\
         .route(\"/api/items\", get(list).post(create))\n\
         let budget(\"/x\") = map.get(\"key\");\n",
    );
    let labels: Vec<_> = found
        .iter()
        .map(|route| (route.label(), route.line))
        .collect();
    assert_eq!(
        labels,
        [
            ("GET /".to_string(), 1),
            ("POST /posts".to_string(), 1),
            ("GET /api/items".to_string(), 2),
            ("POST /api/items".to_string(), 2),
        ]
    );
}

#[test]
fn headers_need_the_server_baseline_or_a_layer_and_a_strict_csp() {
    let (_root, sources, config) = load(&[("src/main.rs", "fn main() { serve(router()); }\n")]);
    let check = headers(&sources, &config);
    assert_eq!(check.status, EvidenceStatus::Findings(1));
    assert!(messages(&check)[0].contains("Core header baseline is not active"));

    let (_root, sources, config) = load(&[(
        "src/main.rs",
        "fn main() { Server::new(router()).run(3000); }\n",
    )]);
    assert_eq!(
        headers(&sources, &config).status,
        EvidenceStatus::NoFindings
    );

    let (_root, sources, config) = load(&[
        (
            "src/main.rs",
            "fn main() { app.layer(from_fn(headers_middleware)); }\n",
        ),
        (
            "Rullst.toml",
            "[security]\ncsp = \"default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval' *\"\ncoep = \"unsafe-none\"\n",
        ),
    ]);
    let check = headers(&sources, &config);
    let found = messages(&check);
    assert_eq!(check.status, EvidenceStatus::Findings(4), "{found:?}");
    assert!(found[0].starts_with("Rullst.toml:2 the script policy allows 'unsafe-inline'"));
    assert!(
        found
            .iter()
            .any(|message| message.contains("broad source `*`"))
    );
    assert!(
        found
            .iter()
            .any(|message| message.contains("'unsafe-eval'"))
    );
    assert!(
        found
            .iter()
            .any(|message| message.starts_with("Rullst.toml:3 `coep"))
    );
}

#[test]
fn csp_problems_cover_empty_missing_and_strict_policies() {
    assert_eq!(csp_problems("  ").len(), 1);
    assert!(csp_problems("img-src 'self'")[0].contains("neither `script-src`"));
    assert!(
        csp_problems(crate::generators::audit_report::catalog::GUIDE).len() == 1,
        "a URL is not a policy"
    );
    assert!(
        csp_problems("default-src 'self'; script-src 'self' 'nonce-{NONCE}'; object-src 'none'")
            .is_empty()
    );
    assert!(csp_problems("default-src https:")[0].contains("`https:`"));
}

#[test]
fn csrf_requires_server_or_middleware_for_write_routes_except_signed_webhooks() {
    let routes = "fn routes() { routes![post(\"/posts\" => store), post(\"/webhooks/stripe\" => hook), get(\"/\" => index)] }\n";
    let (_root, sources, config) = load(&[
        ("src/lib.rs", routes),
        (
            "Rullst.toml",
            "[security]\ncsrf_signed_webhook_paths = [\"/webhooks/stripe\"]\n",
        ),
    ]);
    let check = csrf(&sources, &config);
    assert_eq!(check.status, EvidenceStatus::Findings(1));
    assert!(messages(&check)[0].starts_with("src/lib.rs:1 `POST /posts` has no recognized CSRF"));
    assert!(check.detail.contains("/webhooks/stripe"));

    for protection in [
        "fn main() { Server::new(routes()).run(3000); }\n",
        "fn app() { routes().layer(from_fn(rullst::security::csrf_middleware)); }\n",
    ] {
        let (_root, sources, config) = load(&[("src/lib.rs", routes), ("src/main.rs", protection)]);
        assert_eq!(csrf(&sources, &config).status, EvidenceStatus::NoFindings);
    }
    // A comment is not protection.
    let (_root, sources, config) = load(&[(
        "src/lib.rs",
        &format!("{routes}// csrf_middleware is mounted elsewhere\n"),
    )]);
    assert_eq!(csrf(&sources, &config).status, EvidenceStatus::Findings(2));
}

#[test]
fn cookies_need_secure_httponly_and_samesite() {
    let (_root, sources, config) = load(&[
        (
            "src/auth.rs",
            "fn set() { let a = \"app_session=abc; Path=/\"; let b = \"theme=dark; Path=/; SameSite=Lax; Secure\"; }\nfn builder() { Cookie::build(\"sid\").secure(false); }\n",
        ),
        ("Rullst.toml", "[security]\ncsrf_same_site = \"None\"\n"),
    ]);
    let check = cookies(&sources, &config);
    let found = messages(&check);
    assert_eq!(check.status, EvidenceStatus::Findings(3), "{found:?}");
    assert_eq!(
        found[0],
        "src/auth.rs:1 cookie `app_session` is set without Secure, HttpOnly, SameSite"
    );
    assert_eq!(found[1], "src/auth.rs:2 a cookie builder disables Secure");
    assert!(found[2].starts_with("Rullst.toml:2 `csrf_same_site = \"None\"`"));

    let (_root, sources, config) = load(&[(
        "src/auth.rs",
        "fn set() -> String { format!(\"app_session={}; Path=/; HttpOnly; SameSite=Lax{}\", v, s) }\nfn login() { rullst_auth::make_login_cookie(1); }\n",
    )]);
    let check = cookies(&sources, &config);
    assert_eq!(check.status, EvidenceStatus::NoFindings);
    assert!(check.detail.contains("make_login_cookie`: used"));
}

#[test]
fn credential_routes_need_a_rate_limit_or_login_jail() {
    let login =
        "fn routes() { routes![post(\"/login\" => login), post(\"/authors\" => create)] }\n";
    let (_root, sources, _config) = load(&[("src/lib.rs", login)]);
    let check = rate_limit(&sources);
    assert_eq!(check.status, EvidenceStatus::Findings(1));
    assert!(messages(&check)[0].contains("`POST /login`"));

    for limiter in [
        "static LIMIT: LazyLock<RateLimiter> = LazyLock::new(make);\n",
        "fn guard() { let jail = LoginGuard::default(); }\n",
    ] {
        let (_root, sources, _config) = load(&[("src/lib.rs", login), ("src/limit.rs", limiter)]);
        assert_eq!(rate_limit(&sources).status, EvidenceStatus::NoFindings);
    }

    let (_root, sources, _config) = load(&[(
        "src/lib.rs",
        "fn routes() { routes![get(\"/login\" => form)] }\n",
    )]);
    assert!(matches!(
        rate_limit(&sources).status,
        EvidenceStatus::NotChecked(_)
    ));
}

#[test]
fn an_unparsable_configuration_is_an_error_not_a_clean_result() {
    let (_root, sources, config) = load(&[
        ("src/main.rs", "fn main() { Server::new(r); }\n"),
        ("Rullst.toml", "[security\ncsp = \n"),
    ]);
    for check in [
        headers(&sources, &config),
        csrf(&sources, &config),
        cookies(&sources, &config),
    ] {
        assert!(matches!(check.status, EvidenceStatus::Error(_)));
    }
}

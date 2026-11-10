// src/blueprints/security_tests.rs — The generated `src/security_tests.rs`:
// offline checks of a starter's real router behind the staging/production
// baseline (headers, WAF, CSRF), the SaaS starter's sign-in rate limit, and a
// smoke test of every parameterless page (`PAGES`: status, security headers
// and HTML hygiene). The entry point declares the module under `#[cfg(test)]`.

const COMMON: &str = include_str!("security_tests/common.rs.template");
const CSRF: &str = include_str!("security_tests/csrf.rs.template");
const RATE_LIMIT: &str = include_str!("security_tests/rate_limit.rs.template");
const API: &str = include_str!("security_tests/api.rs.template");
const SMOKE: &str = include_str!("security_tests/smoke.rs.template");

/// Path of the generated test module.
pub(super) const PATH: &str = "src/security_tests.rs";

/// Starters with HTTP routes, each checked against its own write routes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Starter {
    Blank,
    BlankApi,
    Saas,
}

/// The write the CSRF check sends, the status a valid token yields and why.
struct CsrfProbe {
    path: &'static str,
    accepted: &'static str,
    reason: &'static str,
}

impl Starter {
    fn csrf_probe(self) -> Option<CsrfProbe> {
        let probe = |path, accepted, reason| {
            Some(CsrfProbe {
                path,
                accepted,
                reason,
            })
        };
        match self {
            // The bearer-token check replaces the browser CSRF check.
            Self::BlankApi => None,
            Self::Blank => probe(
                "/clicked",
                "OK",
                "The page's own form sends the token, so the counter write succeeds.",
            ),
            Self::Saas => probe(
                "/logout",
                "SEE_OTHER",
                "With the token, sign-out reaches its handler and redirects.",
            ),
        }
    }

    /// The generated `PAGES` entries: every parameterless GET page with the
    /// status a signed-out visitor receives. Sign-in pages redirect to
    /// `/login` (303).
    fn pages(self) -> &'static [(&'static str, u16)] {
        match self {
            Self::Blank | Self::BlankApi => &[("/", 200), ("/health", 200)],
            Self::Saas => &[
                ("/", 200),
                ("/pricing", 200),
                ("/login", 200),
                ("/register", 200),
                ("/password/forgot", 200),
                ("/dashboard", 303),
                ("/password/change", 303),
                ("/reports/billing", 303),
                ("/health", 200),
            ],
        }
    }

    fn app_body(self) -> &'static str {
        match self {
            Self::Blank => {
                "    rullst::security::apply_security_baseline(\n        super::router().into_axum(),\n        security_config(),\n        Environment::Production,\n    )\n    .expect(\"security baseline\")"
            }
            Self::BlankApi => {
                "    let endpoints = super::machine_endpoints(TEST_API_TOKEN).expect(\"machine endpoints\");\n    rullst::security::apply_security_baseline_with_machine_endpoints(\n        super::router().into_axum(),\n        security_config(),\n        Environment::Production,\n        endpoints,\n    )\n    .expect(\"security baseline\")"
            }
            Self::Saas => {
                "    let router = super::router().expect(\"router builds offline\");\n    rullst::security::apply_security_baseline(\n        router.into_axum(),\n        security_config(),\n        Environment::Production,\n    )\n    .expect(\"security baseline\")"
            }
        }
    }
}

/// Source of the generated `src/security_tests.rs` for `starter`.
pub(super) fn source(starter: Starter) -> String {
    let mut source = COMMON.replace("__APP__", starter.app_body());
    if let Some(probe) = starter.csrf_probe() {
        source.push_str(
            &CSRF
                .replace("__PATH__", probe.path)
                .replace("__ACCEPTED__", probe.accepted)
                .replace("__REASON__", probe.reason),
        );
    }
    if starter == Starter::Saas {
        source.push_str(RATE_LIMIT);
    }
    if starter == Starter::BlankApi {
        source.push_str(API);
    }
    let pages = starter
        .pages()
        .iter()
        .map(|(path, status)| format!("    (\"{path}\", {status}),\n"))
        .collect::<String>();
    source.push_str(&SMOKE.replace("__PAGES__", &pages));
    source
}

#[cfg(test)]
mod tests {
    use super::*;

    const STARTERS: [Starter; 3] = [Starter::Blank, Starter::BlankApi, Starter::Saas];

    #[test]
    fn every_starter_gets_parsable_tests_without_placeholders() {
        for starter in STARTERS {
            let source = source(starter);
            syn::parse_file(&source).unwrap_or_else(|error| panic!("{starter:?}: {error}"));
            assert!(!source.contains("__"), "{starter:?} keeps a placeholder");
            assert!(source.contains("fn production_responses_carry_the_security_headers()"));
            assert!(source.contains("fn the_waf_refuses_injection_and_accepts_prose()"));
            assert!(source.contains("Environment::Production"));
            assert!(source.contains("fn every_page_answers_with_safe_headers_and_sound_html()"));
            assert!(source.contains("rullst::testing::TestApp::new(app())"));
            assert!(source.contains(".assert_security_headers();"));
            assert!(source.contains(".assert_html_hygiene();"));
            // Every assertion carries a comment saying what it protects.
            let lines = source.lines().collect::<Vec<_>>();
            for (index, line) in lines.iter().enumerate() {
                if line.trim_start().starts_with("assert") {
                    assert!(
                        lines[index - 1].trim_start().starts_with("//"),
                        "{starter:?}: uncommented assertion `{line}`"
                    );
                }
            }
        }
    }

    #[test]
    fn pages_list_every_parameterless_get_route_once() {
        for starter in STARTERS {
            let source = source(starter);
            let list = source
                .split_once("const PAGES: &[(&str, u16)] = &[\n")
                .and_then(|(_, rest)| rest.split_once("];"))
                .map(|(list, _)| list)
                .unwrap_or_else(|| panic!("{starter:?}: no PAGES list"));
            let expected = starter
                .pages()
                .iter()
                .map(|(path, status)| format!("    (\"{path}\", {status}),\n"))
                .collect::<String>();
            assert_eq!(list, expected, "{starter:?}");
            let paths = starter.pages().iter().map(|(path, _)| *path).collect::<Vec<_>>();
            let mut unique = paths.clone();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), paths.len(), "{starter:?}");
            assert!(paths.contains(&"/") && paths.contains(&"/health"));
            for (path, status) in starter.pages() {
                assert!(path.starts_with('/') && !path.contains(['{', ':', '?']));
                assert!(matches!(status, 200 | 303), "{starter:?} {path}");
            }
        }
        // The SaaS router mounts each listed page; its parameterised or
        // POST-only routes stay out of the list.
        let saas = include_str!("saas/src/main.rs.template");
        for (path, status) in Starter::Saas.pages() {
            let mounted = saas.contains(&format!("get(\"{path}\" =>"))
                || saas.contains(&format!(".route(\"{path}\", rullst::routing::get("))
                || *path == "/health";
            assert!(mounted, "{path} is not a GET route of the SaaS starter");
            let protected = saas.contains(&format!(
                ".route(\"{path}\", rullst::routing::get(controllers::"
            ));
            assert_eq!(*status == 303, protected, "{path}");
        }
        assert!(!Starter::Saas.pages().iter().any(|(path, _)| *path == "/password/reset"));
    }

    #[test]
    fn checks_follow_each_starters_routes() {
        let has = |starter, test: &str| source(starter).contains(test);
        for starter in STARTERS {
            assert_eq!(
                has(starter, "fn repeated_sign_in_attempts_are_rate_limited()"),
                starter == Starter::Saas,
                "{starter:?}"
            );
            assert_eq!(
                has(starter, "fn csrf_refuses_a_write_without_the_token()"),
                starter != Starter::BlankApi,
                "{starter:?}"
            );
        }
        assert!(has(
            Starter::BlankApi,
            "fn json_writes_need_the_api_token()"
        ));
        assert!(has(
            Starter::BlankApi,
            "apply_security_baseline_with_machine_endpoints("
        ));
    }
}

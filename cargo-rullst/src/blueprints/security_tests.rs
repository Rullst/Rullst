// src/blueprints/security_tests.rs — The generated `src/security_tests.rs`:
// offline checks of a starter's real router behind the staging/production
// baseline (headers, WAF, CSRF), plus the starter's rate limit and IDOR guard.
// The entry point declares the module under `#[cfg(test)]`.

const COMMON: &str = include_str!("security_tests/common.rs.template");
const CSRF: &str = include_str!("security_tests/csrf.rs.template");
const RATE_LIMIT: &str = include_str!("security_tests/rate_limit.rs.template");
const LMS_IDOR: &str = include_str!("security_tests/lms_idor.rs.template");
const API: &str = include_str!("security_tests/api.rs.template");

/// Path of the generated test module.
pub(super) const PATH: &str = "src/security_tests.rs";

/// Starters with HTTP routes, each checked against its own write routes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Starter {
    Blank,
    BlankApi,
    Blog,
    Portfolio,
    Saas,
    Lms,
    Erp,
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
            Self::Blog | Self::Portfolio => probe(
                "/",
                "METHOD_NOT_ALLOWED",
                "With the token the write passes CSRF; this starter has no write route yet.",
            ),
            Self::Saas | Self::Lms => probe(
                "/logout",
                "SEE_OTHER",
                "With the token, sign-out reaches its handler and redirects.",
            ),
            Self::Erp => probe(
                "/products",
                "UNSUPPORTED_MEDIA_TYPE",
                "With the token the write reaches the handler, which rejects the empty form.",
            ),
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
            _ => {
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
    if matches!(starter, Starter::Saas | Starter::Lms) {
        source.push_str(RATE_LIMIT);
    }
    if starter == Starter::Lms {
        source.push_str(LMS_IDOR);
    }
    if starter == Starter::BlankApi {
        source.push_str(API);
    }
    source
}

#[cfg(test)]
mod tests {
    use super::*;

    const STARTERS: [Starter; 7] = [
        Starter::Blank,
        Starter::BlankApi,
        Starter::Blog,
        Starter::Portfolio,
        Starter::Saas,
        Starter::Lms,
        Starter::Erp,
    ];

    #[test]
    fn every_starter_gets_parsable_tests_without_placeholders() {
        for starter in STARTERS {
            let source = source(starter);
            syn::parse_file(&source).unwrap_or_else(|error| panic!("{starter:?}: {error}"));
            assert!(!source.contains("__"), "{starter:?} keeps a placeholder");
            assert!(source.contains("fn production_responses_carry_the_security_headers()"));
            assert!(source.contains("fn the_waf_refuses_injection_and_accepts_prose()"));
            assert!(source.contains("Environment::Production"));
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
    fn checks_follow_each_starters_routes() {
        let has = |starter, test: &str| source(starter).contains(test);
        for starter in STARTERS {
            assert_eq!(
                has(starter, "fn repeated_sign_in_attempts_are_rate_limited()"),
                matches!(starter, Starter::Saas | Starter::Lms),
                "{starter:?}"
            );
            assert_eq!(
                has(starter, "fn another_learners_lessons_are_refused()"),
                starter == Starter::Lms,
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

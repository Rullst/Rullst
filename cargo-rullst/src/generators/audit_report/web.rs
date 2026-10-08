//! Static checks of the browser-facing baseline: security headers and CSP,
//! CSRF, cookie attributes and rate limiting on authentication routes.

use crate::generators::audit_compliance::EvidenceStatus;

use super::catalog::{COOKIES, CSRF, HEADERS, RATE_LIMIT};
use super::model::{Check, Finding};
use super::sources::{ProjectSources, SecurityConfig};

/// The configuration as loaded: absent, parsed, or unreadable.
pub(super) type Config = Result<Option<SecurityConfig>, String>;

const SERVER_EVIDENCE: [&str; 2] = ["Server::new(", "Server::new_hot("];
const HEADER_LAYERS: [&str; 3] = [
    "headers_middleware",
    "SecureHeadersLayer",
    "CspSecurityLayer",
];
const NO_SOURCE: &str = "no src directory was available to inspect";

fn uses_server(sources: &ProjectSources) -> bool {
    sources.first_mention(&SERVER_EVIDENCE).is_some()
}

fn config_error(spec: &'static super::catalog::Spec, error: &str) -> Check {
    Check::with_status(
        spec,
        EvidenceStatus::Error(error.to_string()),
        Vec::new(),
        error,
    )
}

pub(super) fn headers(sources: &ProjectSources, config: &Config) -> Check {
    if !sources.available {
        return Check::not_checked(&HEADERS, NO_SOURCE);
    }
    let config = match config {
        Ok(config) => config.as_ref(),
        Err(error) => return config_error(&HEADERS, error),
    };
    let server = uses_server(sources);
    let layer = sources.first_mention(&HEADER_LAYERS);
    let mut findings = Vec::new();
    if !server && layer.is_none() {
        findings.push(Finding::new(
            "src",
            None,
            "no `Server` and no header layer (`headers_middleware`, `SecureHeadersLayer`) was found; the Core header baseline is not active",
        ));
    }
    let mut csp_source = "default (strict, nonce-based)";
    if let Some(config) = config {
        if let Some(csp) = config.string("csp") {
            csp_source = "custom `[security] csp`";
            let line = config.line_of("csp");
            findings.extend(
                csp_problems(csp)
                    .into_iter()
                    .map(|problem| Finding::new("Rullst.toml", line, problem)),
            );
        }
        if let Some(coep) = config.string("coep") {
            let line = config.line_of("coep");
            match coep.trim() {
                "require-corp" | "credentialless" => {}
                "unsafe-none" => findings.push(Finding::new(
                    "Rullst.toml",
                    line,
                    "`coep = \"unsafe-none\"` disables cross-origin embedder isolation; keep it only as a reviewed decision",
                )),
                _ => findings.push(Finding::new(
                    "Rullst.toml",
                    line,
                    "`coep` is not one of require-corp, credentialless or unsafe-none",
                )),
            }
        }
    }
    let detail = format!(
        "`Server` baseline: {}; router header layer: {}; CSP: {csp_source}. The Core baseline sends HSTS, nosniff, frame and CSP headers in staging and production only; response-level tests against the deployment remain required.",
        if server { "found" } else { "not found" },
        layer.map_or("not found".to_string(), |layer| format!("`{layer}`")),
    );
    Check::from_findings(&HEADERS, findings, detail)
}

/// Weaknesses of a CSP value.
pub(super) fn csp_problems(csp: &str) -> Vec<String> {
    if csp.trim().is_empty() {
        return vec!["the CSP is empty, so no content restriction is sent".to_string()];
    }
    let directives: Vec<(String, Vec<&str>)> = csp
        .split(';')
        .filter_map(|directive| {
            let mut parts = directive.split_whitespace();
            let name = parts.next()?.to_ascii_lowercase();
            Some((name, parts.collect()))
        })
        .collect();
    let sources = |name: &str| {
        directives
            .iter()
            .find(|(directive, _)| directive == name)
            .map(|(_, values)| values.clone())
    };
    let mut problems = Vec::new();
    let script = sources("script-src").or_else(|| sources("default-src"));
    match script {
        None => problems.push(
            "the CSP has neither `script-src` nor `default-src`, so scripts are unrestricted"
                .to_string(),
        ),
        Some(values) => {
            if values.contains(&"'unsafe-inline'") {
                problems.push("the script policy allows 'unsafe-inline'".to_string());
            }
            if let Some(wildcard) = values
                .iter()
                .find(|value| matches!(**value, "*" | "http:" | "https:" | "data:"))
            {
                problems.push(format!(
                    "the script policy allows the broad source `{wildcard}`"
                ));
            }
        }
    }
    if directives
        .iter()
        .any(|(_, values)| values.contains(&"'unsafe-eval'"))
    {
        problems.push("the CSP allows 'unsafe-eval'".to_string());
    }
    problems
}

pub(super) fn csrf(sources: &ProjectSources, config: &Config) -> Check {
    if !sources.available {
        return Check::not_checked(&CSRF, NO_SOURCE);
    }
    let config = match config {
        Ok(config) => config.as_ref(),
        Err(error) => return config_error(&CSRF, error),
    };
    let server = uses_server(sources);
    let middleware = sources.mentions("csrf_middleware");
    let machine = sources.mentions("with_machine_endpoints");
    let exempt = config
        .map(|config| config.strings("csrf_signed_webhook_paths"))
        .unwrap_or_default();
    let routes = sources.routes();
    let writes: Vec<_> = routes.iter().filter(|(_, route)| route.writes()).collect();
    let mut findings = Vec::new();
    if !server && !middleware {
        for (file, route) in &writes {
            if exempt.contains(&route.path) {
                continue;
            }
            findings.push(Finding::new(
                file.display.clone(),
                Some(route.line),
                format!(
                    "`{}` has no recognized CSRF protection: neither the `Server` baseline nor `csrf_middleware` was found",
                    route.label()
                ),
            ));
        }
    }
    let protection = match (server, middleware) {
        (true, true) => {
            "the `Server` baseline (staging/production) and `csrf_middleware` on the router"
        }
        (true, false) => "the `Server` baseline (staging and production only)",
        (false, true) => "`csrf_middleware` on the router",
        (false, false) => "none recognized",
    };
    let detail = format!(
        "{} write route(s) recognized on single lines; protection: {protection}; machine endpoints (`with_machine_endpoints`): {}; signed-webhook CSRF exemptions: {}. Exempt paths must be authenticated by a mandatory signature middleware.",
        writes.len(),
        if machine { "configured" } else { "none" },
        if exempt.is_empty() {
            "none".to_string()
        } else {
            exempt.join(", ")
        },
    );
    Check::from_findings(&CSRF, findings, detail)
}

/// Cookie names whose value authenticates a user.
fn sensitive_cookie(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    !name.contains("csrf")
        && !name.contains("xsrf")
        && [
            "session", "sess", "sid", "auth", "token", "jwt", "remember", "login",
        ]
        .iter()
        .any(|marker| name.contains(marker))
}

/// A string literal that looks like a `Set-Cookie` value: `name=value; Attr`.
/// Returns the name and the lowercase attributes.
fn cookie_literal(literal: &str) -> Option<(&str, String)> {
    let (first, attributes) = literal.split_once(';')?;
    let (name, _) = first.split_once('=')?;
    let name = name.trim();
    let token = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'));
    let lower = attributes.to_ascii_lowercase();
    let attribute = [
        "path=", "max-age=", "expires=", "httponly", "samesite", "domain=", "secure",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    (token && attribute).then_some((name, lower))
}

fn string_literals(line: &str) -> Vec<&str> {
    line.split('"').skip(1).step_by(2).collect()
}

pub(super) fn cookies(sources: &ProjectSources, config: &Config) -> Check {
    if !sources.available {
        return Check::not_checked(&COOKIES, NO_SOURCE);
    }
    let config = match config {
        Ok(config) => config.as_ref(),
        Err(error) => return config_error(&COOKIES, error),
    };
    let mut findings = Vec::new();
    let mut constructed = 0usize;
    for file in &sources.files {
        for (index, line) in file.code.lines().enumerate() {
            let at = Some(index + 1);
            for literal in string_literals(line) {
                let Some((name, lower)) = cookie_literal(literal) else {
                    continue;
                };
                constructed += 1;
                let mut missing = Vec::new();
                // A format placeholder may append `; Secure` outside development.
                if !lower.contains("secure") && !literal.contains('{') {
                    missing.push("Secure");
                }
                if sensitive_cookie(name) && !lower.contains("httponly") {
                    missing.push("HttpOnly");
                }
                if !lower.contains("samesite") {
                    missing.push("SameSite");
                }
                if !missing.is_empty() {
                    findings.push(Finding::new(
                        file.display.clone(),
                        at,
                        format!("cookie `{name}` is set without {}", missing.join(", ")),
                    ));
                }
            }
            for (pattern, problem) in [
                (".secure(false)", "a cookie builder disables Secure"),
                (".http_only(false)", "a cookie builder disables HttpOnly"),
                ("SameSite::None", "a cookie uses SameSite=None"),
            ] {
                if line.contains(pattern) {
                    constructed += 1;
                    findings.push(Finding::new(file.display.clone(), at, problem));
                }
            }
        }
    }
    let same_site = config.and_then(|config| config.string("csrf_same_site"));
    if let (Some(config), Some(value)) = (config, same_site) {
        let line = config.line_of("csrf_same_site");
        if value.eq_ignore_ascii_case("none") {
            findings.push(Finding::new(
                "Rullst.toml",
                line,
                "`csrf_same_site = \"None\"` sends the CSRF cookie on cross-site requests; use Lax or Strict",
            ));
        } else if !value.eq_ignore_ascii_case("lax") && !value.eq_ignore_ascii_case("strict") {
            findings.push(Finding::new(
                "Rullst.toml",
                line,
                "`csrf_same_site` is not Lax, Strict or None",
            ));
        }
    }
    let framework = sources.mentions("make_login_cookie");
    let detail = format!(
        "{constructed} cookie construction(s) recognized in production source; CSRF cookie SameSite: {}; `rullst_auth::make_login_cookie`: {}. Framework session and CSRF cookies add Secure in staging and production; HttpOnly is expected only on cookies scripts must not read.",
        same_site.unwrap_or("Lax (default)"),
        if framework {
            "used (HttpOnly, SameSite=Lax, Secure outside development)"
        } else {
            "not used"
        },
    );
    Check::from_findings(&COOKIES, findings, detail)
}

/// Whether a path segment names an authentication action.
fn auth_route(path: &str) -> bool {
    path.split('/').any(|segment| {
        let segment = segment.to_ascii_lowercase();
        matches!(segment.as_str(), "auth" | "oauth")
            || [
                "login",
                "logon",
                "signin",
                "sign-in",
                "sign_in",
                "register",
                "signup",
                "sign-up",
                "sign_up",
                "password",
                "forgot",
                "otp",
                "mfa",
                "2fa",
                "magic-link",
                "session",
                "token",
            ]
            .iter()
            .any(|prefix| segment.starts_with(prefix))
    })
}

const LIMITERS: [&str; 6] = [
    "RedisRateLimiter",
    "RateLimiter",
    "rate_limit",
    "LoginGuard",
    "GovernorLayer",
    "RateLimitLayer",
];

pub(super) fn rate_limit(sources: &ProjectSources) -> Check {
    if !sources.available {
        return Check::not_checked(&RATE_LIMIT, NO_SOURCE);
    }
    let routes = sources.routes();
    let credential: Vec<_> = routes
        .iter()
        .filter(|(_, route)| route.writes() && auth_route(&route.path))
        .collect();
    if credential.is_empty() {
        return Check::not_checked(
            &RATE_LIMIT,
            "no authentication write route was recognized on a single line",
        );
    }
    let limiter = sources.first_mention(&LIMITERS);
    let findings = if limiter.is_some() {
        Vec::new()
    } else {
        credential
            .iter()
            .map(|(file, route)| {
                Finding::new(
                    file.display.clone(),
                    Some(route.line),
                    format!(
                        "`{}` has no recognized rate limit or login jail in this project",
                        route.label()
                    ),
                )
            })
            .collect()
    };
    let detail = format!(
        "{} authentication write route(s) recognized; limiter evidence: {}. Evidence is project-wide: it does not prove each route is mounted behind it.",
        credential.len(),
        limiter.map_or("none".to_string(), |limiter| format!("`{limiter}`")),
    );
    Check::from_findings(&RATE_LIMIT, findings, detail)
}

#[cfg(test)]
#[path = "web_tests.rs"]
mod tests;

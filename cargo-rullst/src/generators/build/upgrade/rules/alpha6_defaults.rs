//! The 13.0.0-alpha.6 default changes that application code can name: the
//! health probes no longer report the framework version, the CSRF
//! middleware decides static files by path (`csrf_static_prefixes` and
//! `csrf_static_paths`) instead of by extension, the CSRF token a page
//! embeds is masked per response, and `Server` applies the CSRF check and the
//! secure headers in every environment. The static-file rule compares paths
//! with the lists of the project's `Rullst.toml` ([`StaticPaths`]).
//! [`super::tokens`] calls the field, string,
//! method and call rules with what it reads, [`super::rust`] the struct rule with
//! each struct literal and [`super::files`] the script rule with each script
//! line.

use super::Rule;
use super::catalog::{
    CSRF_COOKIE_IN_PAGE, CSRF_STATIC_PREFIXES, HEALTH_FRAMEWORK_VERSION,
    HEALTH_PROBE_VERSION_FIELD, ROUTER_BASELINE_LAYERS,
};
use proc_macro2::{TokenStream, TokenTree};
use rullst_core::config::SecurityConfig;

/// The probe payload types whose `version` field 13.0.0-alpha.6 removed.
const PROBE_TYPES: &[&str] = &["HealthProbeResponse", "ReadinessProbeResponse"];

fn names_probe_type(text: &str) -> bool {
    PROBE_TYPES.iter().any(|name| text.contains(name))
}

/// The rule for a field access `.name` (not a method call): `.version` in a
/// file that names a probe payload type.
pub(super) fn field_rule(name: &str, text: &str) -> Option<&'static Rule> {
    (name == "version" && names_probe_type(text)).then_some(&HEALTH_PROBE_VERSION_FIELD)
}

/// The rule for a struct literal whose path ends in `last`: a
/// `HealthProbeResponse`, which is `#[non_exhaustive]` and has no `version`.
pub(super) fn struct_rule(last: &str) -> Option<&'static Rule> {
    (last == "HealthProbeResponse").then_some(&HEALTH_PROBE_VERSION_FIELD)
}

/// The rule for a string literal: the key `"version"` in a file that also
/// names one of Rullst's probe paths in a string, such as a test that reads
/// `body["version"]` after `get("/health")`. The path ends the string
/// (`"/health"`, `"{base}/ready"`), so an application's own `/healthz` does
/// not count.
pub(super) fn string_rule(value: &str, text: &str) -> Option<&'static Rule> {
    (value == "version" && (text.contains("/health\"") || text.contains("/ready\"")))
        .then_some(&HEALTH_FRAMEWORK_VERSION)
}

/// The `[security]` `csrf_static_prefixes` and `csrf_static_paths` that
/// `V13-CSRF-STATIC-PREFIXES` compares paths with: those of the project's
/// `Rullst.toml`, read as `Server` reads them
/// ([`super::files::csrf_static_paths`]), or Core's defaults.
#[derive(Debug, Default)]
pub(super) struct StaticPaths {
    security: SecurityConfig,
    /// Why the project's `Rullst.toml` was not used, the note of each
    /// location; the defaults apply then.
    unread: Option<String>,
}

impl StaticPaths {
    /// The lists of a `Rullst.toml` that `Server` accepts: a missing key
    /// keeps its default and a list replaces it.
    pub(super) fn project(security: SecurityConfig) -> Self {
        Self {
            security,
            unread: None,
        }
    }

    /// The defaults, for a `Rullst.toml` that `Server` refuses with `error`.
    pub(super) fn unread(error: impl std::fmt::Display) -> Self {
        Self {
            security: SecurityConfig::default(),
            unread: Some(format!(
                "`Rullst.toml` was not used ({error}), so this path is compared with the default `csrf_static_prefixes` and `csrf_static_paths`; `Server` refuses to start with it"
            )),
        }
    }

    /// The note of every location when `Rullst.toml` was not used.
    pub(super) fn note(&self) -> Option<&str> {
        self.unread.as_deref()
    }

    /// Whether the lists cover the files a `nest_service` mounted at `path`
    /// serves below it: only a prefix does (`/static` and `/static/images`
    /// below `/static/`), because an exact path, even `/assets/`, matches no
    /// file below the mount.
    fn covers_mount(&self, path: &str) -> bool {
        let below = format!("{}/", path.trim_end_matches('/'));
        self.security
            .csrf_static_prefixes
            .iter()
            .any(|prefix| below.starts_with(prefix.as_str()))
    }

    /// Whether the lists cover a route at `path` (`route_service`, `.route`,
    /// `routes!`), as `Server` checks the request path: below a prefix or
    /// equal to one exact path.
    fn covers_route(&self, path: &str) -> bool {
        self.security.is_csrf_static_path(path)
    }
}

/// The extensions that made a `GET` or `HEAD` path a static file until
/// 13.0.0-alpha.5, which compared the end of the path exactly.
const ALPHA5_STATIC_EXTENSIONS: [&str; 12] = [
    ".txt", ".xml", ".ico", ".json", ".css", ".js", ".png", ".jpg", ".jpeg", ".svg", ".webp",
    ".wasm",
];

/// Whether `path`, a route's path, ends in an extension that alpha.5 treated
/// as a static file.
fn static_extension(path: &str) -> bool {
    ALPHA5_STATIC_EXTENSIONS
        .iter()
        .any(|extension| path.ends_with(extension))
}

/// Whether `tokens`, groups included, contain the identifier `name`.
fn names(tokens: &[TokenTree], name: &str) -> bool {
    tokens.iter().any(|tree| match tree {
        TokenTree::Ident(ident) => ident == name,
        TokenTree::Group(group) => names(&group.stream().into_iter().collect::<Vec<_>>(), name),
        _ => false,
    })
}

/// The rule for a method call with `args`: `.nest_service("/path", ..)`
/// below no prefix of the `static_paths` or `.route_service("/path", ..)` at
/// a path they do not cover, in a file that names `ServeDir` or `ServeFile`,
/// and the routes of [`route_rule`].
pub(super) fn method_rule(
    name: &str,
    args: &TokenStream,
    text: &str,
    static_paths: &StaticPaths,
) -> Option<&'static Rule> {
    let tokens: Vec<TokenTree> = args.clone().into_iter().collect();
    match name {
        "route" => route_rule(&tokens, static_paths),
        "nest_service" | "route_service"
            if text.contains("ServeDir") || text.contains("ServeFile") =>
        {
            let path = super::tokens::string_value(tokens.first())?;
            let covered = if name == "nest_service" {
                static_paths.covers_mount(&path)
            } else {
                static_paths.covers_route(&path)
            };
            (path.starts_with('/') && !covered).then_some(&CSRF_STATIC_PREFIXES)
        }
        _ => None,
    }
}

/// The rule for `.route("/path", method_router)` at a path the
/// `static_paths` do not cover: a `get_service` of a `ServeDir` or
/// `ServeFile` at any such path, and a `get` route whose path ends in an
/// extension alpha.5 treated as static (`/feed.xml`, `/data.json`), whose
/// response, a file or what its handler renders, never minted the CSRF
/// cookie until then.
fn route_rule(tokens: &[TokenTree], static_paths: &StaticPaths) -> Option<&'static Rule> {
    let path = super::tokens::string_value(tokens.first())?;
    let router = &tokens[1..];
    let file_service =
        names(router, "get_service") && (names(router, "ServeDir") || names(router, "ServeFile"));
    let static_route = names(router, "get") && static_extension(&path);
    (path.starts_with('/') && (file_service || static_route) && !static_paths.covers_route(&path))
        .then_some(&CSRF_STATIC_PREFIXES)
}

/// The rule for a call `path(args)`: a `get("/path" => handler)` route of
/// `routes!` whose path ends in an extension alpha.5 treated as static and
/// the `static_paths` do not cover, as [`route_rule`] reads `.route`.
pub(super) fn call_rule(
    segments: &[&str],
    args: &[TokenTree],
    static_paths: &StaticPaths,
) -> Option<&'static Rule> {
    if segments.last() != Some(&"get") {
        return None;
    }
    let path = super::tokens::string_value(args.first())?;
    let arrow = matches!(args.get(1), Some(TokenTree::Punct(punct)) if punct.as_char() == '=');
    (arrow && path.starts_with('/') && static_extension(&path) && !static_paths.covers_route(&path))
        .then_some(&CSRF_STATIC_PREFIXES)
}

/// The rule for a method call with `args`: `.layer(..)` or `.route_layer(..)`
/// whose argument names Core's `csrf_middleware` or `headers_middleware`,
/// which `Server` applies in every environment since 13.0.0-alpha.6.
pub(super) fn layer_rule(name: &str, args: &TokenStream) -> Option<&'static Rule> {
    if !matches!(name, "layer" | "route_layer") {
        return None;
    }
    let mut pending = vec![args.clone()];
    while let Some(tokens) = pending.pop() {
        for token in tokens {
            match token {
                proc_macro2::TokenTree::Group(group) => pending.push(group.stream()),
                proc_macro2::TokenTree::Ident(ident)
                    if ident == "csrf_middleware" || ident == "headers_middleware" =>
                {
                    return Some(&ROUTER_BASELINE_LAYERS);
                }
                _ => {}
            }
        }
    }
    None
}

/// The rule for a string literal: the CSRF cookie's name, `"rullst_csrf"`
/// or `"rullst_csrf="`, which code uses to read the raw cookie value (the
/// `.strip_prefix("rullst_csrf=")` of the `get_csrf_token` helper generated
/// before 13.0.0-alpha.6, an application's own `cookie(headers,
/// "rullst_csrf")`, `jar.get("rullst_csrf")` or `c.name() == "rullst_csrf"`),
/// in a file that does not call `mask_csrf_token`. A literal that only
/// contains the name, such as the `format!("rullst_csrf={token}")` of a
/// request that sends the cookie, is not a read. It is a rule about
/// production behaviour, so test code is skipped ([`super::test_code`]).
pub(super) fn cookie_rule(value: &str, text: &str) -> Option<&'static Rule> {
    (matches!(value, "rullst_csrf" | "rullst_csrf=") && !text.contains("mask_csrf_token"))
        .then_some(&CSRF_COOKIE_IN_PAGE)
}

/// A script line that reads `version` from a `/health` or `/ready` response
/// on the same line, such as `curl -fsS $URL/health | jq -r .version`.
pub(super) fn reads_probe_version(line: &str) -> bool {
    let probe = line.contains("/health") || line.contains("/ready");
    let field = line.match_indices(".version").any(|(index, found)| {
        line[index + found.len()..]
            .chars()
            .next()
            .is_none_or(|next| !(next.is_ascii_alphanumeric() || next == '_'))
    });
    probe && (field || line.contains("\"version\"") || line.contains("'version'"))
}

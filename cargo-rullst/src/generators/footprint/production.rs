//! `--monitor-url`: what the application served in production, per route,
//! read from its monitoring endpoint (`GET /_rullst/monitor/v1`, mounted by
//! `Server::with_monitor`), next to the lab figures of the run.
//!
//! The monitor token comes from [`TOKEN_ENV`], never from the command line,
//! and is sent only over HTTPS (plain HTTP only to a loopback address, such
//! as an SSH tunnel), never after a redirect. The document holds only route
//! patterns and counters; footprint keeps those and nothing else.

use super::render::bytes;
use crate::term::{Style, sanitize};
use serde::Serialize;
use std::net::IpAddr;
use std::time::Duration;

/// The environment variable holding the monitor token.
pub(super) const TOKEN_ENV: &str = "RULLST_MONITOR_TOKEN";
/// The monitor's path, appended to a URL given as an origin.
pub(super) const MONITOR_PATH: &str = "/_rullst/monitor/v1";
/// The only document version read.
const SCHEMA: &str = "rullst.monitor.v1";
/// Larger documents are not read.
const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
/// Routes kept from a document (the monitor reports at most 257).
const MAX_ROUTES: usize = 512;
/// Characters kept of a route pattern, method or application label.
const MAX_LABEL_CHARS: usize = 256;
/// Routes the terminal report lists.
const SHOWN_ROUTES: usize = 5;
const TIMEOUT: Duration = Duration::from_secs(10);

fn loopback(url: &reqwest::Url) -> bool {
    let host = url.host_str().unwrap_or_default();
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    match host.trim_start_matches('[').trim_end_matches(']').parse() {
        Ok(IpAddr::V4(address)) => address.is_loopback(),
        Ok(IpAddr::V6(address)) => {
            address.is_loopback() || address.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback())
        }
        Err(_) => false,
    }
}

/// Accepts the application's HTTPS origin (the monitor path is appended) or
/// the monitor URL itself; plain HTTP only on loopback. Credentials, a query
/// or a fragment are refused, so nothing secret sits in the URL.
pub(crate) fn validate_monitor_url(raw: &str) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(raw.trim())
        .map_err(|_| format!("`{}` is not a valid URL", sanitize(raw)))?;
    match url.scheme() {
        "https" => {}
        "http" if loopback(&url) => {}
        "http" => {
            return Err(
                "the monitor URL must use https:// (plain http:// only on a loopback address); the token is never sent in the clear"
                    .to_string(),
            );
        }
        _ => return Err("the monitor URL must use https://".to_string()),
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(format!(
            "the monitor URL must not contain credentials, a query or a fragment; the token comes from {TOKEN_ENV}"
        ));
    }
    if url.path() == "/" {
        url.set_path(MONITOR_PATH);
    } else if !url.path().ends_with(MONITOR_PATH) {
        return Err(format!(
            "pass the application's origin (https://app.example) or a URL ending in {MONITOR_PATH}"
        ));
    }
    Ok(url)
}

/// One route as the monitor reports it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ProductionRoute {
    pub method: String,
    /// The route pattern, such as `/orders/{id}`.
    pub route: String,
    pub requests_total: u64,
    /// `null` when the application's Rullst does not count bytes.
    pub bytes_total: Option<u64>,
    pub bytes_per_request: Option<f64>,
    pub window_requests: Option<u64>,
    pub window_bytes: Option<u64>,
}

/// The production section of the report.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Production {
    /// `measured`, `not_run` (without `--monitor-url`) or `not_measured`.
    pub status: &'static str,
    pub reason: Option<String>,
    /// The monitor URL, which never holds the token.
    pub source: Option<String>,
    /// The application's name and version, as it reports them.
    pub application: Option<String>,
    pub uptime_seconds: Option<u64>,
    pub window_seconds: Option<u64>,
    /// Whether the application counts response bytes (13.0.0-alpha.5 and
    /// later).
    pub bytes_reported: Option<bool>,
    pub requests_total: Option<u64>,
    pub bytes_total: Option<u64>,
    /// The `GET` route the `--path` page matches, when one does.
    pub path_route: Option<ProductionRoute>,
    pub routes: Vec<ProductionRoute>,
}

impl Production {
    pub(super) fn unmeasured(status: &'static str, reason: impl Into<String>) -> Self {
        Self {
            status,
            reason: Some(reason.into()),
            source: None,
            application: None,
            uptime_seconds: None,
            window_seconds: None,
            bytes_reported: None,
            requests_total: None,
            bytes_total: None,
            path_route: None,
            routes: Vec::new(),
        }
    }
}

/// Whether `path` (without its query) is an instance of the route
/// `pattern`: `{name}` matches one segment and `{*name}` the rest.
pub(super) fn route_matches(pattern: &str, path: &str) -> bool {
    let path = path.split('?').next().unwrap_or_default();
    let mut patterns = pattern.trim_start_matches('/').split('/');
    let mut segments = path.trim_start_matches('/').split('/');
    loop {
        match (patterns.next(), segments.next()) {
            (None, None) => return true,
            (Some(pattern), _) if pattern.starts_with("{*") && pattern.ends_with('}') => {
                return true;
            }
            (Some(pattern), Some(segment))
                if pattern.starts_with('{') && pattern.ends_with('}') =>
            {
                if segment.is_empty() {
                    return false;
                }
            }
            (Some(pattern), Some(segment)) if pattern == segment => {}
            _ => return false,
        }
    }
}

fn label(value: &serde_json::Value) -> Option<String> {
    let text = value.as_str()?;
    Some(sanitize(
        &text.chars().take(MAX_LABEL_CHARS).collect::<String>(),
    ))
}

fn route(value: &serde_json::Value) -> Option<ProductionRoute> {
    let requests_total = value.get("requests_total")?.as_u64()?;
    let bytes_total = value.get("bytes_total").and_then(serde_json::Value::as_u64);
    let window = value.get("window");
    Some(ProductionRoute {
        method: label(value.get("method")?)?,
        route: label(value.get("route")?)?,
        requests_total,
        bytes_total,
        bytes_per_request: bytes_total
            .filter(|_| requests_total > 0)
            .map(|bytes| bytes as f64 / requests_total as f64),
        window_requests: window
            .and_then(|window| window.get("requests"))
            .and_then(serde_json::Value::as_u64),
        window_bytes: window
            .and_then(|window| window.get("bytes"))
            .and_then(serde_json::Value::as_u64),
    })
}

/// The production section from a monitor `document`, read from `source`;
/// `path` is the run's `--path`.
pub(super) fn from_document(document: &serde_json::Value, source: &str, path: &str) -> Production {
    if document.get("schema").and_then(serde_json::Value::as_str) != Some(SCHEMA) {
        return Production::unmeasured(
            "not_measured",
            format!("the monitor did not answer with a {SCHEMA} document"),
        );
    }
    let http = document.get("http");
    let routes: Vec<ProductionRoute> = http
        .and_then(|http| http.get("routes"))
        .and_then(serde_json::Value::as_array)
        .map(|routes| routes.iter().take(MAX_ROUTES).filter_map(route).collect())
        .unwrap_or_default();
    let path_route = routes
        .iter()
        .filter(|candidate| candidate.method == "GET" && route_matches(&candidate.route, path))
        .min_by_key(|candidate| candidate.route.matches('{').count())
        .cloned();
    let application = document.get("application").and_then(|application| {
        let name = label(application.get("name")?)?;
        let version = label(application.get("version")?)?;
        Some(format!("{name} {version}"))
    });
    let bytes_total = http
        .and_then(|http| http.get("bytes_total"))
        .and_then(serde_json::Value::as_u64);
    Production {
        status: "measured",
        reason: None,
        source: Some(source.to_string()),
        application,
        uptime_seconds: document
            .get("uptime_seconds")
            .and_then(serde_json::Value::as_u64),
        window_seconds: http
            .and_then(|http| http.get("window_seconds"))
            .and_then(serde_json::Value::as_u64),
        bytes_reported: Some(bytes_total.is_some()),
        requests_total: http
            .and_then(|http| http.get("requests_total"))
            .and_then(serde_json::Value::as_u64),
        bytes_total,
        path_route,
        routes,
    }
}

/// Why the monitor's answer `status` carries no document.
fn refused(status: u16) -> String {
    let hint = match status {
        401 => format!("check {TOKEN_ENV}"),
        403 => "the monitor refused the request: it needs HTTPS (or a trusted proxy reporting it) and no Origin header".to_string(),
        404 => "the application does not mount the monitor: call Server::with_monitor and configure RULLST_MONITOR_TOKENS".to_string(),
        429 => "the monitor's request budget or failed-token lockout applies; try again later".to_string(),
        _ => "see the application's logs".to_string(),
    };
    format!("the monitor answered {status}; {hint}")
}

/// Reads the monitor at `url` with `token`; `path` is the run's `--path`.
pub(super) async fn read(url: &reqwest::Url, token: Option<&str>, path: &str) -> Production {
    let Some(token) = token.map(str::trim).filter(|token| !token.is_empty()) else {
        return Production::unmeasured(
            "not_measured",
            format!(
                "set {TOKEN_ENV} to one of the application's monitor tokens; it is never read from the command line"
            ),
        );
    };
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(TIMEOUT)
        .build();
    let Ok(client) = client else {
        return Production::unmeasured("not_measured", "the HTTP client could not be built");
    };
    let response = client
        .get(url.clone())
        .bearer_auth(token)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await;
    let mut response = match response {
        Ok(response) => response,
        Err(_) => {
            return Production::unmeasured("not_measured", format!("nothing answered on {url}"));
        }
    };
    let status = response.status().as_u16();
    if status != 200 {
        return Production::unmeasured("not_measured", refused(status));
    }
    let mut body = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) if body.len() + chunk.len() <= MAX_DOCUMENT_BYTES => {
                body.extend_from_slice(&chunk);
            }
            Ok(Some(_)) => {
                return Production::unmeasured(
                    "not_measured",
                    "the monitor document is larger than 4 MiB",
                );
            }
            Ok(None) => break,
            Err(_) => {
                return Production::unmeasured("not_measured", "the monitor's answer ended early");
            }
        }
    }
    match serde_json::from_slice::<serde_json::Value>(&body) {
        Ok(document) => from_document(&document, url.as_str(), path),
        Err(_) => Production::unmeasured("not_measured", "the monitor's answer is not JSON"),
    }
}

/// The production section of the terminal report, with the lab's bytes per
/// request beside the `--path` route; empty without `--monitor-url`.
pub(super) fn render(
    production: &Production,
    lab_bytes_per_request: Option<f64>,
    style: Style,
) -> String {
    if production.status == "not_run" {
        return String::new();
    }
    let mut out = format!("{}\n", style.bold("Production (monitor)"));
    if production.status != "measured" {
        out.push_str(&format!(
            "  {}\n\n",
            style.yellow(&format!(
                "NOT MEASURED  {}",
                production.reason.as_deref().unwrap_or_default()
            ))
        ));
        return out;
    }
    let uptime = production
        .uptime_seconds
        .map_or_else(String::new, |seconds| format!(", up {seconds} s"));
    out.push_str(&format!(
        "  Source        {}{}\n",
        production.application.as_deref().unwrap_or("application"),
        style.dim(&format!(
            "  {}{uptime}",
            production.source.as_deref().unwrap_or_default()
        ))
    ));
    if production.bytes_reported != Some(true) {
        out.push_str(&format!(
            "  {}\n\n",
            style.yellow("Bytes         NOT REPORTED  the application's Rullst counts no response bytes (13.0.0-alpha.5 and later do)")
        ));
        return out;
    }
    out.push_str(&format!(
        "  Served        {}  {}\n",
        bytes(production.bytes_total.unwrap_or_default()),
        style.dim(&format!(
            "{} requests since start, bodies as sent after compression",
            production.requests_total.unwrap_or_default()
        ))
    ));
    if let Some(route) = &production.path_route {
        let lab = lab_bytes_per_request.map_or_else(
            || "lab: pass --bench to compare".to_string(),
            |lab| format!("lab: {}/request", bytes(lab.round() as u64)),
        );
        out.push_str(&format!(
            "  This page     {}/request  {}\n",
            bytes(route.bytes_per_request.unwrap_or_default().round() as u64),
            style.dim(&format!(
                "{} {} in production; {lab}",
                route.method, route.route
            ))
        ));
    }
    let mut busiest: Vec<&ProductionRoute> = production.routes.iter().collect();
    busiest.sort_by(|left, right| {
        right
            .bytes_total
            .cmp(&left.bytes_total)
            .then_with(|| left.route.cmp(&right.route))
    });
    for route in busiest.into_iter().take(SHOWN_ROUTES) {
        out.push_str(&format!(
            "  {:<13} {}  {}\n",
            "Route",
            bytes(route.bytes_total.unwrap_or_default()),
            style.dim(&format!(
                "{} {}: {} requests, {}/request",
                route.method,
                route.route,
                route.requests_total,
                route
                    .bytes_per_request
                    .map_or_else(|| "-".to_string(), |each| bytes(each.round() as u64))
            ))
        ));
    }
    out.push('\n');
    out
}

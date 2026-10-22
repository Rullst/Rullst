//! The monitor route: request admission and the document handler.
//!
//! Admission order: a browser cross-origin request (`Origin`, or a
//! cross-site or same-site `Sec-Fetch-Site`) is refused with `403`; in
//! staging and production a request without TLS evidence gets `426`; then
//! the client's request budget (`429`) and failed-credential lockout (`429`,
//! credentials not evaluated); then a monitor token, then the application
//! authorizer. Missing and invalid credentials get the same `401`.

use super::auth::ErasedAuthorizer;
use super::collector::Collector;
use super::payload::{Application, Document, Pending, QueueReport};
use super::token::{Presented, TokenSet, presented};
use super::{
    MAX_MONITOR_QUEUES, MONITOR_PATH, Monitor, MonitorAccess, MonitorError, MonitorVerifiedTls,
};
use crate::config::Environment;
use crate::queue::Queue;
use crate::radar::RadarSnapshot;
use crate::resilience::{RateLimitConfig, RateLimiter, peer_rate_limit_key};
use crate::security::{ClientAddr, ForwardedProto};
use axum::Router;
use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

/// Requests a client may burst; one more is allowed every second.
const REQUEST_BURST: f64 = 30.0;
const REQUEST_REFILL_PER_SECOND: f64 = 1.0;
/// Presented invalid tokens a client may send before it is locked out;
/// afterwards one more attempt is allowed every [`LOCKOUT_SECONDS`].
const FAILURE_BURST: f64 = 10.0;
const LOCKOUT_SECONDS: u64 = 90;
/// Longest wait for one queue's pending count.
pub(super) const QUEUE_PROBE_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_APPLICATION_BYTES: usize = 128;
const MAX_QUEUE_NAME_BYTES: usize = 64;

/// A validated monitor, ready to be mounted by the server.
pub(crate) struct Installed {
    state: Arc<EndpointState>,
}

struct EndpointState {
    tokens: TokenSet,
    authorizer: Option<Arc<dyn ErasedAuthorizer>>,
    application: Option<Application>,
    queues: Vec<(String, Arc<Queue>)>,
    environment: Environment,
    loopback_plaintext: bool,
    requests: RateLimiter,
    failures: RateLimiter,
    collector: Arc<Collector>,
}

impl Installed {
    /// Validates `monitor`. `Ok(None)` means no credential is configured, so
    /// nothing is mounted.
    pub(crate) fn prepare(
        monitor: Monitor,
        environment: Environment,
    ) -> Result<Option<Self>, MonitorError> {
        let tokens = TokenSet::new(&monitor.tokens)?;
        let application = monitor
            .application
            .map(|(name, version)| validate_application(name, version))
            .transpose()?;
        validate_queues(&monitor.queues)?;
        if tokens.is_empty() && monitor.authorizer.is_none() {
            return Ok(None);
        }
        Ok(Some(Self {
            state: Arc::new(EndpointState {
                tokens,
                authorizer: monitor.authorizer,
                application,
                queues: monitor.queues,
                environment,
                loopback_plaintext: monitor.loopback_plaintext,
                requests: RateLimiter::new(RateLimitConfig::new(
                    REQUEST_BURST,
                    REQUEST_REFILL_PER_SECOND,
                )),
                failures: RateLimiter::new(RateLimitConfig::new(
                    FAILURE_BURST,
                    1.0 / LOCKOUT_SECONDS as f64,
                )),
                collector: Arc::new(Collector::new()),
            }),
        }))
    }

    /// Adds the monitor route. Call it before the framework layers, so they
    /// wrap it like any application route.
    pub(crate) fn mount(&self, router: Router) -> Router {
        router.route(
            MONITOR_PATH,
            axum::routing::get(serve)
                .with_state(Arc::clone(&self.state))
                .layer(axum::middleware::map_response(protect)),
        )
    }

    /// Adds the recording layer. Call it after the framework layers.
    pub(crate) fn record(&self, router: Router) -> Router {
        super::record::record_requests(router, Arc::clone(&self.state.collector))
    }

    #[cfg(test)]
    pub(super) fn collector(&self) -> &Collector {
        &self.state.collector
    }
}

fn validate_application(name: String, version: String) -> Result<Application, MonitorError> {
    let valid = |value: &str| {
        !value.is_empty()
            && value.len() <= MAX_APPLICATION_BYTES
            && !value.chars().any(char::is_control)
    };
    if valid(&name) && valid(&version) {
        Ok(Application { name, version })
    } else {
        Err(MonitorError::InvalidApplication)
    }
}

fn validate_queues(queues: &[(String, Arc<Queue>)]) -> Result<(), MonitorError> {
    if queues.len() > MAX_MONITOR_QUEUES {
        return Err(MonitorError::TooManyQueues {
            maximum: MAX_MONITOR_QUEUES,
        });
    }
    for (index, (name, _)) in queues.iter().enumerate() {
        let valid = !name.is_empty()
            && name.len() <= MAX_QUEUE_NAME_BYTES
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
        if !valid {
            return Err(MonitorError::InvalidQueueName);
        }
        if queues.iter().take(index).any(|(other, _)| other == name) {
            return Err(MonitorError::DuplicateQueue);
        }
    }
    Ok(())
}

async fn serve(State(state): State<Arc<EndpointState>>, request: Request) -> Response {
    if is_cross_origin(request.headers()) {
        return refusal(StatusCode::FORBIDDEN);
    }
    if !state.transport_allowed(&request) {
        let mut response = refusal(StatusCode::UPGRADE_REQUIRED);
        response.headers_mut().insert(
            header::UPGRADE,
            HeaderValue::from_static("TLS/1.2, HTTP/1.1"),
        );
        return response;
    }
    let (parts, _body) = request.into_parts();
    if let Some(refusal) = state.refuse(&parts).await {
        return refusal;
    }
    let document = state.document().await;
    match serde_json::to_vec(&document) {
        Ok(body) => {
            let mut response = Response::new(Body::from(body));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            response
        }
        Err(_) => refusal(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// A browser request from a page: the endpoint is a machine contract, so an
/// application CORS allowlist must not release it to any origin.
fn is_cross_origin(headers: &HeaderMap) -> bool {
    headers.contains_key(header::ORIGIN)
        || headers.get("sec-fetch-site").is_some_and(|site| {
            let site = site.as_bytes();
            site.eq_ignore_ascii_case(b"cross-site") || site.eq_ignore_ascii_case(b"same-site")
        })
}

impl EndpointState {
    /// Plain HTTP is accepted only outside staging and production, with the
    /// application's TLS marker, with HTTPS reported by a trusted proxy, or
    /// (when allowed) from a direct loopback client.
    fn transport_allowed(&self, request: &Request) -> bool {
        let extensions = request.extensions();
        !self.environment.requires_secure_defaults()
            || extensions.get::<MonitorVerifiedTls>().is_some()
            || extensions
                .get::<ClientAddr>()
                .and_then(ClientAddr::forwarded_proto)
                .is_some_and(ForwardedProto::is_https)
            // A loopback peer without forwarding headers: never a client that
            // a (trusted) proxy forwarded.
            || (self.loopback_plaintext && crate::server::dev_telemetry::is_local(request))
    }

    /// The refusal for a request that is not admitted; `None` admits it.
    async fn refuse(&self, parts: &Parts) -> Option<Response> {
        let client = client_key(parts);
        if !self.requests.check_and_consume(&client) {
            return Some(too_many_requests(1));
        }
        if !self.failures.has_token(&client) {
            return Some(too_many_requests(LOCKOUT_SECONDS));
        }
        let presented = presented(&parts.headers);
        if let Presented::Bearer(token) = presented
            && self.tokens.matches(token)
        {
            return None;
        }
        if let Some(authorizer) = &self.authorizer {
            match authorizer.authorize_erased(parts).await {
                Ok(MonitorAccess::Allow) => return None,
                Ok(MonitorAccess::Deny) => return Some(refusal(StatusCode::FORBIDDEN)),
                Ok(MonitorAccess::SignIn) => {}
                Err(error) => {
                    tracing::error!(
                        reason = error.reason(),
                        "monitor authorizer failed; request denied"
                    );
                    return Some(refusal(StatusCode::SERVICE_UNAVAILABLE));
                }
            }
        }
        if matches!(presented, Presented::Bearer(_) | Presented::Invalid) {
            self.failures.check_and_consume(&client);
        }
        Some(unauthorized())
    }

    async fn document(&self) -> Document {
        let radar = RadarSnapshot::collect_async().await;
        let queues = queue_reports(&self.queues).await;
        Document::new(
            self.application.clone(),
            radar,
            &self.collector,
            crate::server::dev_telemetry::orm_unavailable_reason(),
            queues,
        )
    }
}

/// The client's limiter key: the address resolved through trusted proxies,
/// else the socket peer (IPv4 address or IPv6 /64).
fn client_key(parts: &Parts) -> String {
    let extensions = &parts.extensions;
    extensions
        .get::<ClientAddr>()
        .map(ClientAddr::ip)
        .or_else(|| {
            extensions
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ConnectInfo(peer)| peer.ip())
        })
        .or_else(|| {
            extensions
                .get::<MockConnectInfo<SocketAddr>>()
                .map(|MockConnectInfo(peer)| peer.ip())
        })
        .map_or_else(|| "missing-peer-address".to_string(), peer_rate_limit_key)
}

/// Probes every queue concurrently, each within [`QUEUE_PROBE_TIMEOUT`].
pub(super) async fn queue_reports(queues: &[(String, Arc<Queue>)]) -> Vec<QueueReport> {
    let mut probes = tokio::task::JoinSet::new();
    for (index, (_, queue)) in queues.iter().enumerate() {
        let queue = Arc::clone(queue);
        probes.spawn(async move {
            let pending =
                match tokio::time::timeout(QUEUE_PROBE_TIMEOUT, queue.pending_count()).await {
                    Ok(Ok(count)) => Pending::Observed { count },
                    // Driver errors can name hosts or paths; only a fixed reason leaves.
                    Ok(Err(_)) => Pending::Unavailable {
                        reason: "driver_error",
                    },
                    Err(_) => Pending::Unavailable { reason: "timeout" },
                };
            (index, pending)
        });
    }
    let mut pending = vec![
        Pending::Unavailable {
            reason: "probe_failed"
        };
        queues.len()
    ];
    while let Some(probe) = probes.join_next().await {
        if let Ok((index, observed)) = probe
            && let Some(slot) = pending.get_mut(index)
        {
            *slot = observed;
        }
    }
    queues
        .iter()
        .zip(pending)
        .map(|((name, queue), pending)| {
            QueueReport::new(name, pending, queue.activity().snapshot())
        })
        .collect()
}

/// Headers on every response of the monitor route, refusals and `405`
/// included.
async fn protect(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn refusal(status: StatusCode) -> Response {
    status.into_response()
}

fn unauthorized() -> Response {
    let mut response = refusal(StatusCode::UNAUTHORIZED);
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static("Bearer realm=\"rullst-monitor\""),
    );
    response
}

fn too_many_requests(retry_after_seconds: u64) -> Response {
    let mut response = refusal(StatusCode::TOO_MANY_REQUESTS);
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from(retry_after_seconds));
    response
}

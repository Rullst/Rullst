//! Server-Sent Events delivery of realtime channel messages.
//!
//! [`RealtimeSse`] turns a [`BroadcastManager`] channel subscription into a
//! `text/event-stream` response: each [`RealtimeMessage`] becomes one event
//! named after [`RealtimeMessage::event`] whose data is the payload, which the
//! htmx SSE extension swaps into the page (`hx-ext="sse"`,
//! `sse-connect="/events"`, `sse-swap="<event>"`). A stream is one-way and
//! carries no commands; the route that returns it authenticates and
//! authorizes the request like any other `GET`.

use super::{
    BroadcastManager, MAX_EVENT_BYTES, MAX_PAYLOAD_BYTES, RealtimeError, RealtimeMessage,
    validate_name,
};
use axum::http::{HeaderName, HeaderValue, StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, broadcast, watch};
use tokio::time::Instant;

const KEEP_ALIVE_RANGE: (Duration, Duration) = (Duration::from_secs(1), Duration::from_secs(300));
const LIFETIME_RANGE: (Duration, Duration) =
    (Duration::from_secs(1), Duration::from_secs(24 * 60 * 60));
const MAX_CONNECTIONS: usize = 65_536;
/// Seconds a refused browser is asked to wait before reconnecting.
const RETRY_AFTER_SECONDS: &str = "5";

/// Bounds of the Server-Sent Events connections one [`RealtimeSse`] serves.
///
/// Every setter clamps its value into the documented range, so a
/// configuration is always valid.
///
/// Added in v13.0.0-alpha.5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RealtimeSseConfig {
    keep_alive: Duration,
    max_lifetime: Duration,
    max_connections: usize,
    max_payload_bytes: usize,
}

impl Default for RealtimeSseConfig {
    fn default() -> Self {
        Self {
            keep_alive: Duration::from_secs(15),
            max_lifetime: Duration::from_secs(30 * 60),
            max_connections: 1_024,
            max_payload_bytes: MAX_PAYLOAD_BYTES,
        }
    }
}

impl RealtimeSseConfig {
    /// The defaults: a keep-alive comment every 15 seconds, connections that
    /// last at most 30 minutes, 1,024 open connections and 64 KiB payloads.
    pub fn new() -> Self {
        Self::default()
    }

    /// Interval between `: keep-alive` comments on an idle stream, clamped to
    /// 1 second–5 minutes. Keep it below the idle timeout of every proxy in
    /// front of the application.
    pub fn with_keep_alive(mut self, interval: Duration) -> Self {
        self.keep_alive = interval.clamp(KEEP_ALIVE_RANGE.0, KEEP_ALIVE_RANGE.1);
        self
    }

    /// Longest time one connection stays open, clamped to 1 second–24 hours.
    /// The stream then ends and the browser reconnects, so the route checks
    /// the session and authorization again.
    pub fn with_max_lifetime(mut self, lifetime: Duration) -> Self {
        self.max_lifetime = lifetime.clamp(LIFETIME_RANGE.0, LIFETIME_RANGE.1);
        self
    }

    /// Connections open at once across every clone of the [`RealtimeSse`],
    /// clamped to 1–65,536. A request beyond it is refused with
    /// `503 Service Unavailable` and `Retry-After`.
    pub fn with_max_connections(mut self, connections: usize) -> Self {
        self.max_connections = connections.clamp(1, MAX_CONNECTIONS);
        self
    }

    /// Largest payload, in bytes, sent as one event, clamped to 1 byte–64 KiB
    /// (the limit [`super::TenantRealtime::publish`] enforces). A larger
    /// message is skipped for this connection.
    pub fn with_max_payload_bytes(mut self, bytes: usize) -> Self {
        self.max_payload_bytes = bytes.clamp(1, MAX_PAYLOAD_BYTES);
        self
    }

    /// Interval between keep-alive comments.
    pub fn keep_alive(&self) -> Duration {
        self.keep_alive
    }

    /// Longest lifetime of one connection.
    pub fn max_lifetime(&self) -> Duration {
        self.max_lifetime
    }

    /// Connections open at once.
    pub fn max_connections(&self) -> usize {
        self.max_connections
    }

    /// Largest payload sent as one event.
    pub fn max_payload_bytes(&self) -> usize {
        self.max_payload_bytes
    }
}

/// Bounded Server-Sent Events responses for realtime channels.
///
/// Clones share the connection limit and the shutdown signal. Each stream
/// holds one broadcast receiver: the channel's fixed queue is the only
/// per-connection buffer, and a connection that falls further behind than
/// the queue holds is ended instead of skipping messages silently, so the
/// browser reconnects and loads the current state. A stream also ends when
/// its channel closes, after [`RealtimeSseConfig::max_lifetime`] and on
/// [`Self::shutdown`]. Messages whose event name is not 1–128 bytes of ASCII
/// letters, digits, `-`, `_`, `.`, `:` or `/`, or whose payload is larger than
/// [`RealtimeSseConfig::max_payload_bytes`], are skipped.
///
/// Delivery is best effort and process-local, like [`BroadcastManager`]:
/// publish complete HTML fragments (the current state of an element) rather
/// than changes that depend on earlier events.
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use rullst_core::realtime::{BroadcastManager, RealtimeSse, RealtimeSseConfig};
/// use rullst_core::web::axum::{Router, extract::State, response::Response, routing::get};
///
/// #[derive(Clone)]
/// struct AppState {
///     channels: Arc<BroadcastManager>,
///     sse: RealtimeSse,
/// }
///
/// // The page subscribes with `<div hx-ext="sse" sse-connect="/events"
/// // sse-swap="stats">`, after loading `/static/htmx-ext-sse-1.9.12.js`.
/// async fn events(State(state): State<AppState>) -> Response {
///     state.sse.channel(&state.channels, "dashboard")
/// }
///
/// # async fn run() {
/// let state = AppState {
///     channels: Arc::new(BroadcastManager::new()),
///     sse: RealtimeSse::new(RealtimeSseConfig::new()),
/// };
/// let app: Router = Router::new().route("/events", get(events)).with_state(state.clone());
/// // Elsewhere: state.channels.publish("dashboard", "stats", "<p id=\"stats\">42</p>");
/// // On shutdown, before the server waits for open requests: state.sse.shutdown();
/// # let _ = app;
/// # }
/// ```
///
/// Added in v13.0.0-alpha.5.
#[derive(Clone, Debug)]
pub struct RealtimeSse {
    config: RealtimeSseConfig,
    admission: Arc<Semaphore>,
    shutdown: Arc<watch::Sender<bool>>,
}

impl RealtimeSse {
    /// Creates the handler; it mounts no route.
    pub fn new(config: RealtimeSseConfig) -> Self {
        let (shutdown, _) = watch::channel(false);
        Self {
            admission: Arc::new(Semaphore::new(config.max_connections)),
            config,
            shutdown: Arc::new(shutdown),
        }
    }

    /// The limits this handler applies.
    pub fn config(&self) -> RealtimeSseConfig {
        self.config
    }

    /// Streams the messages published to `channel` of `manager` from now on.
    ///
    /// Answers `503 Service Unavailable` with `Retry-After` when the
    /// connection limit is reached or after [`Self::shutdown`]; no channel is
    /// opened then. The caller authorizes the channel for the request.
    pub fn channel(&self, manager: &BroadcastManager, channel: &str) -> Response {
        match self.admit() {
            Some(permit) => self.respond(permit, manager.get_or_create(channel).subscribe()),
            None => unavailable(),
        }
    }

    /// Streams the messages an existing subscription receives, for example
    /// one from [`super::TenantRealtime::subscribe`]. Refused like
    /// [`Self::channel`].
    pub fn subscribe(&self, receiver: broadcast::Receiver<RealtimeMessage>) -> Response {
        match self.admit() {
            Some(permit) => self.respond(permit, receiver),
            None => unavailable(),
        }
    }

    /// Ends every open stream and refuses new ones. Call it when the server
    /// starts draining (for example in the future passed to
    /// `Server::run_with_shutdown`), so that open streams do not hold the
    /// graceful shutdown until its timeout. It cannot be undone.
    pub fn shutdown(&self) {
        self.shutdown.send_replace(true);
    }

    /// Whether [`Self::shutdown`] was called on this handler or a clone.
    pub fn is_shut_down(&self) -> bool {
        *self.shutdown.borrow()
    }

    /// Streams open now, across every clone.
    pub fn open_connections(&self) -> usize {
        self.config
            .max_connections
            .saturating_sub(self.admission.available_permits())
    }

    fn admit(&self) -> Option<OwnedSemaphorePermit> {
        if self.is_shut_down() {
            return None;
        }
        Arc::clone(&self.admission).try_acquire_owned().ok()
    }

    fn respond(
        &self,
        permit: OwnedSemaphorePermit,
        receiver: broadcast::Receiver<RealtimeMessage>,
    ) -> Response {
        let connection = Connection {
            receiver,
            shutdown: self.shutdown.subscribe(),
            _shutdown_sender: Arc::clone(&self.shutdown),
            deadline: Instant::now() + self.config.max_lifetime,
            max_payload_bytes: self.config.max_payload_bytes,
            _permit: permit,
        };
        let stream = futures_util::stream::unfold(connection, next_event);
        let keep_alive = KeepAlive::new()
            .interval(self.config.keep_alive)
            .text("keep-alive");
        let mut response = Sse::new(stream).keep_alive(keep_alive).into_response();
        // Ask buffering reverse proxies (nginx) to forward each event at once.
        response.headers_mut().insert(
            HeaderName::from_static("x-accel-buffering"),
            HeaderValue::from_static("no"),
        );
        response
    }
}

/// The state of one open stream; dropping it releases the connection slot.
struct Connection {
    receiver: broadcast::Receiver<RealtimeMessage>,
    shutdown: watch::Receiver<bool>,
    /// Keeps the signal alive after every [`RealtimeSse`] clone is dropped,
    /// so that only [`RealtimeSse::shutdown`] ends the stream.
    _shutdown_sender: Arc<watch::Sender<bool>>,
    deadline: Instant,
    max_payload_bytes: usize,
    _permit: OwnedSemaphorePermit,
}

async fn next_event(mut connection: Connection) -> Option<(Result<Event, Infallible>, Connection)> {
    loop {
        let received = tokio::select! {
            biased;
            _ = connection.shutdown.wait_for(|stopped| *stopped) => return None,
            () = tokio::time::sleep_until(connection.deadline) => return None,
            received = connection.receiver.recv() => received,
        };
        match received {
            Ok(message) => {
                if let Some(event) = event_for(&message, connection.max_payload_bytes) {
                    return Some((Ok(event), connection));
                }
                tracing::debug!(
                    target: "rullst::realtime",
                    payload_bytes = message.payload.len(),
                    "skipped a realtime message that cannot be sent as a server-sent event"
                );
            }
            Err(broadcast::error::RecvError::Lagged(missed)) => {
                tracing::debug!(
                    target: "rullst::realtime",
                    missed,
                    "ended a server-sent event stream that fell behind its channel"
                );
                return None;
            }
            Err(broadcast::error::RecvError::Closed) => return None,
        }
    }
}

/// The event for one message, or `None` when SSE cannot carry it within the
/// limits.
fn event_for(message: &RealtimeMessage, max_payload_bytes: usize) -> Option<Event> {
    if message.payload.len() > max_payload_bytes
        || validate_name(&message.event, MAX_EVENT_BYTES, RealtimeError::InvalidEvent).is_err()
    {
        return None;
    }
    // The validated name has no line break, which `Event::event` rejects;
    // `data` splits the payload's line breaks into `data:` lines itself.
    Some(
        Event::default()
            .event(&message.event)
            .data(&message.payload),
    )
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [(header::RETRY_AFTER, RETRY_AFTER_SECONDS)],
    )
        .into_response()
}

#[cfg(test)]
#[path = "sse_tests.rs"]
mod tests;

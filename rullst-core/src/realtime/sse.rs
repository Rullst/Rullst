//! Server-Sent Events delivery of realtime channel messages.
//!
//! [`RealtimeSse`] turns a [`BroadcastManager`] channel subscription into a
//! `text/event-stream` response: each [`RealtimeMessage`] becomes one event
//! named after [`RealtimeMessage::event`] whose data is the payload, which the
//! htmx SSE extension swaps into the page (`hx-ext="sse"`,
//! `sse-connect="/events"`, `sse-swap="<event>"`). A stream is one-way and
//! carries no commands; the route that returns it authenticates and
//! authorizes the request like any other `GET`.
//!
//! Every response starts with an SSE `retry:` field. It sets how long the
//! browser's `EventSource` waits before reconnecting when the stream ends,
//! and a refused request uses it to send the browser away for a while, which
//! `EventSource` honours (it ignores `Retry-After` and gives up on an error
//! status).

use super::revalidation::{self, NoRevalidation, Revalidate, Revalidator};
use super::{BroadcastManager, MAX_PAYLOAD_BYTES, RealtimeMessage};
use axum::http::{HeaderName, HeaderValue};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures_util::StreamExt;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, broadcast, watch};
use tokio::time::Instant;

const KEEP_ALIVE_RANGE: (Duration, Duration) = (Duration::from_secs(1), Duration::from_secs(300));
const LIFETIME_RANGE: (Duration, Duration) =
    (Duration::from_secs(1), Duration::from_secs(24 * 60 * 60));
const RECONNECT_RANGE: (Duration, Duration) =
    (Duration::from_millis(100), Duration::from_secs(300));
const MAX_CONNECTIONS: usize = 65_536;
/// Shortest delay a refused browser is asked to wait before reconnecting.
const MIN_REFUSED_RETRY: Duration = Duration::from_secs(5);

/// Bounds of the Server-Sent Events connections one [`RealtimeSse`] serves.
///
/// Every setter clamps its value into the documented range, so a
/// configuration is always valid.
///
/// Added in v13.0.0-alpha.5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RealtimeSseConfig {
    keep_alive: Duration,
    reconnect_delay: Duration,
    max_lifetime: Duration,
    max_connections: usize,
    max_payload_bytes: usize,
    revalidation_interval: Duration,
}

impl Default for RealtimeSseConfig {
    fn default() -> Self {
        Self {
            keep_alive: Duration::from_secs(15),
            reconnect_delay: Duration::from_secs(1),
            max_lifetime: Duration::from_secs(30 * 60),
            max_connections: 1_024,
            max_payload_bytes: MAX_PAYLOAD_BYTES,
            revalidation_interval: revalidation::DEFAULT_REVALIDATION_INTERVAL,
        }
    }
}

impl RealtimeSseConfig {
    /// The defaults: a keep-alive comment every 15 seconds, a 1-second
    /// reconnect delay, connections that last at most 30 minutes, 1,024 open
    /// connections and 64 KiB payloads.
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

    /// How long the browser waits before reconnecting after a stream ends
    /// (its lifetime ran out, it fell behind its channel, or [`RealtimeSse::shutdown`]),
    /// clamped to 100 milliseconds–5 minutes. Every stream starts with it as
    /// the SSE `retry:` field; without one, browsers wait about 3 seconds.
    pub fn with_reconnect_delay(mut self, delay: Duration) -> Self {
        self.reconnect_delay = delay.clamp(RECONNECT_RANGE.0, RECONNECT_RANGE.1);
        self
    }

    /// Longest time one connection stays open, clamped to 1 second–24 hours.
    /// The stream then ends and the browser reconnects after
    /// [`Self::with_reconnect_delay`], so the route checks the session and
    /// authorization again.
    pub fn with_max_lifetime(mut self, lifetime: Duration) -> Self {
        self.max_lifetime = lifetime.clamp(LIFETIME_RANGE.0, LIFETIME_RANGE.1);
        self
    }

    /// Connections open at once across every clone of the [`RealtimeSse`],
    /// clamped to 1–65,536. A request beyond it is refused: it receives a
    /// stream that only sets a `retry:` of 5–10 seconds and ends, so the
    /// browser comes back later (see [`RealtimeSse::channel`]).
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

    /// How often a stream opened with [`RealtimeSse::channel_revalidated`]
    /// or [`RealtimeSse::subscribe_revalidated`] checks that its request may
    /// still receive it, clamped to 1 second–5 minutes (15 seconds by
    /// default). This bounds how long a revoked session keeps a stream that
    /// another process revoked.
    ///
    /// Added in v13.0.0-alpha.6.
    pub fn with_revalidation_interval(mut self, interval: Duration) -> Self {
        self.revalidation_interval = revalidation::clamp_interval(interval);
        self
    }

    /// Interval between revalidations of a revalidated stream.
    pub fn revalidation_interval(&self) -> Duration {
        self.revalidation_interval
    }

    /// Interval between keep-alive comments.
    pub fn keep_alive(&self) -> Duration {
        self.keep_alive
    }

    /// Delay the browser waits before reconnecting after a stream ends.
    pub fn reconnect_delay(&self) -> Duration {
        self.reconnect_delay
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
/// starts with a `retry:` field ([`RealtimeSseConfig::reconnect_delay`]) and
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
/// // sse-swap="stats">`, after loading `/static/htmx-ext-sse-2.2.4.js`.
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
    /// When the connection limit is reached, or after [`Self::shutdown`], the
    /// request is refused without opening the channel: it still receives
    /// `200 OK` with `text/event-stream`, but the stream holds only a comment
    /// naming the reason and a `retry:` of a random 5–10 seconds (or between
    /// once and twice [`RealtimeSseConfig::reconnect_delay`], when that is
    /// longer), and then ends. `EventSource` and the htmx SSE extension
    /// honour that delay; on an error status such as
    /// `503 Service Unavailable` they would ignore `Retry-After` and retry on
    /// their own schedule (the htmx extension first within half a second).
    /// The random part keeps browsers refused together from returning
    /// together.
    /// The caller authorizes the channel for the request.
    pub fn channel(&self, manager: &BroadcastManager, channel: &str) -> Response {
        match self.admit() {
            Ok(permit) => self.respond(
                permit,
                manager.get_or_create(channel).subscribe(),
                None::<Revalidator<NoRevalidation>>,
            ),
            Err(reason) => self.refuse(reason),
        }
    }

    /// [`Self::channel`] for a request whose authorization can end while the
    /// stream is open, such as a signed-in session: `check` runs every
    /// [`RealtimeSseConfig::revalidation_interval`] and whenever its
    /// [`Revalidate::changed`] signal fires, and the stream ends when it
    /// answers `false` or takes longer than
    /// [`super::REVALIDATION_DEADLINE`]. The browser then reconnects and the
    /// route authenticates it again.
    ///
    /// Added in v13.0.0-alpha.6.
    pub fn channel_revalidated<R: Revalidate>(
        &self,
        manager: &BroadcastManager,
        channel: &str,
        check: R,
    ) -> Response {
        match self.admit() {
            Ok(permit) => self.respond(
                permit,
                manager.get_or_create(channel).subscribe(),
                Some(Revalidator::new(check, self.config.revalidation_interval)),
            ),
            Err(reason) => self.refuse(reason),
        }
    }

    /// Streams the messages an existing subscription receives, for example
    /// one from [`super::TenantRealtime::subscribe`]. Refused like
    /// [`Self::channel`].
    pub fn subscribe(&self, receiver: broadcast::Receiver<RealtimeMessage>) -> Response {
        match self.admit() {
            Ok(permit) => self.respond(permit, receiver, None::<Revalidator<NoRevalidation>>),
            Err(reason) => self.refuse(reason),
        }
    }

    /// [`Self::subscribe`] revalidated like [`Self::channel_revalidated`].
    ///
    /// Added in v13.0.0-alpha.6.
    pub fn subscribe_revalidated<R: Revalidate>(
        &self,
        receiver: broadcast::Receiver<RealtimeMessage>,
        check: R,
    ) -> Response {
        match self.admit() {
            Ok(permit) => self.respond(
                permit,
                receiver,
                Some(Revalidator::new(check, self.config.revalidation_interval)),
            ),
            Err(reason) => self.refuse(reason),
        }
    }

    /// Ends every open stream and refuses new ones (see [`Self::channel`]).
    /// Call it when the server starts draining (for example in the future
    /// passed to `Server::run_with_shutdown`), so that open streams do not
    /// hold the graceful shutdown until its timeout; their browsers reconnect
    /// after [`RealtimeSseConfig::reconnect_delay`]. It cannot be undone.
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

    fn admit(&self) -> Result<OwnedSemaphorePermit, Refusal> {
        if self.is_shut_down() {
            return Err(Refusal::ShuttingDown);
        }
        Arc::clone(&self.admission)
            .try_acquire_owned()
            .map_err(|_| Refusal::ConnectionLimit)
    }

    /// A stream that only sends the browser away for a while and ends.
    fn refuse(&self, reason: Refusal) -> Response {
        let base = refused_retry_base(self.config.reconnect_delay);
        let extra = rand::random_range(0..=whole_millis(base));
        tracing::debug!(
            target: "rullst::realtime",
            reason = reason.comment(),
            "refused a server-sent event stream"
        );
        let event = Event::default()
            .comment(reason.comment())
            .retry(base + Duration::from_millis(extra));
        let stream = futures_util::stream::iter([Ok::<Event, Infallible>(event)]);
        with_unbuffered_proxy(Sse::new(stream).into_response())
    }

    fn respond<R: Revalidate>(
        &self,
        permit: OwnedSemaphorePermit,
        receiver: broadcast::Receiver<RealtimeMessage>,
        revalidator: Option<Revalidator<R>>,
    ) -> Response {
        let connection = Connection {
            receiver,
            revalidator,
            shutdown: self.shutdown.subscribe(),
            _shutdown_sender: Arc::clone(&self.shutdown),
            deadline: Instant::now() + self.config.max_lifetime,
            max_payload_bytes: self.config.max_payload_bytes,
            _permit: permit,
        };
        // The first event sets how long the browser waits before reconnecting.
        let retry = Event::default().retry(self.config.reconnect_delay);
        let stream = futures_util::stream::once(std::future::ready(Ok(retry)))
            .chain(futures_util::stream::unfold(connection, next_event));
        let keep_alive = KeepAlive::new()
            .interval(self.config.keep_alive)
            .text("keep-alive");
        with_unbuffered_proxy(Sse::new(stream).keep_alive(keep_alive).into_response())
    }
}

/// Why a stream was refused.
#[derive(Clone, Copy)]
enum Refusal {
    ConnectionLimit,
    ShuttingDown,
}

impl Refusal {
    /// The SSE comment a refused stream carries.
    fn comment(self) -> &'static str {
        match self {
            Self::ConnectionLimit => "connection limit reached",
            Self::ShuttingDown => "shutting down",
        }
    }
}

/// The fixed part of a refused request's `retry:`: 5 seconds, or the
/// reconnect delay when that is longer. A random extra of up to the same
/// amount is added.
fn refused_retry_base(reconnect_delay: Duration) -> Duration {
    reconnect_delay.max(MIN_REFUSED_RETRY)
}

fn whole_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[path = "sse_connection.rs"]
mod connection;
use connection::{Connection, next_event};

/// Asks buffering reverse proxies (nginx) to forward each event at once.
fn with_unbuffered_proxy(mut response: Response) -> Response {
    response.headers_mut().insert(
        HeaderName::from_static("x-accel-buffering"),
        HeaderValue::from_static("no"),
    );
    response
}

#[cfg(test)]
#[path = "sse_tests.rs"]
mod tests;

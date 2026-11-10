# Tutorial 34: Realtime Dashboard with Server-Sent Events

Build a dashboard whose numbers update without a page reload. The server
renders each update with `html!` and pushes it over Server-Sent Events (SSE);
the htmx SSE extension swaps it into the page. There is no WebSocket, no
client-side state and no inline script, so the page runs under Rullst's
strict Content Security Policy.

SSE is one-way: the server sends, the browser receives. Clicks and forms still
go to ordinary routes with htmx (`hx-post`). This replaces the Live components
that 13.0.0-alpha.5 removed (see the
[migration guide](../migration-v13.md#upgrading-an-alpha4-application-to-alpha5)).

---

## How the pieces fit

| Piece | Where it lives |
| :--- | :--- |
| `BroadcastManager` | One shared, process-local registry of named channels (`rullst::realtime`). |
| `RealtimeSse` | Turns a channel subscription into a bounded `text/event-stream` response. |
| `static/htmx-ext-sse-1.9.12.js` | The htmx 1.9.12 SSE extension, written by `cargo rullst new` next to htmx. Rullst ships it patched so that its reconnect delay grows after failures (upstream's never did); use this copy rather than the upstream file. |
| `hx-ext="sse"`, `sse-connect`, `sse-swap` | Attributes that open the stream and say which element each event replaces. |

Each message published to the channel becomes one SSE event: its name is the
message's `event` and its data is the `payload`. An element with
`sse-swap="stats"` receives the data of every `stats` event as its new inner
HTML.

---

## Step 1: The state, the fragment and the routes

```rust,no_run
use std::sync::{Arc, Mutex, PoisonError};

use rullst::html;
use rullst::html::Markup;
use rullst::htmx::{HtmxRequest, render_page_with_head};
use rullst::realtime::{BroadcastManager, RealtimeSse, RealtimeSseConfig};
use rullst::response::{Html, Response};
use rullst::security::CspNonce;
use rullst::web::axum::{self, Extension, extract::State, routing::get};

/// The channel the dashboard listens to.
const CHANNEL: &str = "dashboard";
/// The SSE event whose data replaces the statistics.
const STATS_EVENT: &str = "stats";

#[derive(Clone)]
pub struct Dashboard {
    channels: Arc<BroadcastManager>,
    sse: RealtimeSse,
    /// The figures last published, for pages rendered before the next update.
    figures: Arc<Mutex<(u64, u64)>>,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(BroadcastManager::new()),
            sse: RealtimeSse::new(RealtimeSseConfig::new()),
            figures: Arc::new(Mutex::new((0, 0))),
        }
    }

    /// Keeps new figures and sends them to every open dashboard. The channel
    /// itself retains nothing: `publish` fails when no browser is connected.
    pub fn publish(&self, active_users: u64, queued_jobs: u64) {
        *self.figures.lock().unwrap_or_else(PoisonError::into_inner) =
            (active_users, queued_jobs);
        let fragment = stats(active_users, queued_jobs).into_string();
        let _ = self.channels.publish(CHANNEL, STATS_EVENT, &fragment);
    }

    /// The figures last published.
    fn figures(&self) -> (u64, u64) {
        *self.figures.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The part of the page that changes. `html!` escapes every value, so the
/// payload is safe to swap in.
fn stats(active_users: u64, queued_jobs: u64) -> Markup {
    html! {
        <dl class="dashboard__stats">
            <div><dt>"Active users"</dt><dd>{active_users.to_string()}</dd></div>
            <div><dt>"Queued jobs"</dt><dd>{queued_jobs.to_string()}</dd></div>
        </dl>
    }
}

/// The page: the current figures, and a container that opens the stream.
async fn page(
    State(dashboard): State<Dashboard>,
    htmx: HtmxRequest,
    nonce: Option<Extension<CspNonce>>,
) -> Html<String> {
    let (active_users, queued_jobs) = dashboard.figures();
    let nonce = nonce.map(|Extension(nonce)| nonce.as_str().to_owned());
    let head = html! {
        <script nonce?={nonce} src="/static/htmx-ext-sse-1.9.12.js"></script>
    };
    let content = html! {
        <main hx-ext="sse" sse-connect="/dashboard/events">
            <h1>"Operations"</h1>
            <section sse-swap="stats" aria-live="polite">{stats(active_users, queued_jobs)}</section>
        </main>
    };
    render_page_with_head(&htmx, "en", "Operations", head, content)
}

/// The stream. Authenticate and authorize this route like the page itself.
async fn events(State(dashboard): State<Dashboard>) -> Response {
    dashboard.sse.channel(&dashboard.channels, CHANNEL)
}

pub fn dashboard_routes(dashboard: Dashboard) -> rullst::Router {
    let routes = axum::Router::new()
        .route("/dashboard", get(page))
        .route("/dashboard/events", get(events))
        .with_state(dashboard);
    rullst::Router::new().merge_axum(routes)
}
```

`render_page_with_head` loads htmx; the extension script comes after it, from
the same origin and with the request's CSP nonce when the security headers
set one. The page renders the figures last published, so it is complete
before the stream opens and without JavaScript; each `stats` event then
replaces them.

Mount `dashboard_routes(dashboard.clone())` inside the application's
security baseline (CSRF, security headers, authentication) like any other
page.

---

## Step 2: Publish updates

Anything that changes the figures publishes the new fragment: a request
handler after it saves, a queue worker, or a timer. A timer that reads an
application-owned metrics source:

```rust,no_run
# use std::sync::Arc;
# use rullst::realtime::BroadcastManager;
# #[derive(Clone)]
# struct Dashboard { channels: Arc<BroadcastManager> }
# impl Dashboard { fn publish(&self, _: u64, _: u64) {} }
# async fn read_metrics() -> (u64, u64) { (0, 0) }
use std::time::Duration;

fn start_publisher(dashboard: Dashboard) {
    rullst::runtime::spawn(async move {
        let mut tick = rullst::runtime::time::interval(Duration::from_secs(5));
        loop {
            tick.tick().await;
            let (active_users, queued_jobs) = read_metrics().await;
            dashboard.publish(active_users, queued_jobs);
        }
    });
}
```

Publish complete fragments, the current state of an element, rather than
changes that depend on earlier events: a browser that reconnects has missed
whatever was sent meanwhile, and the next full fragment corrects it.

---

## Step 3: Shut the streams down with the server

An SSE response is a request that never ends on its own. Call
`RealtimeSse::shutdown()` when the server starts draining, so open streams
end instead of holding the graceful shutdown until its timeout:

```rust,no_run
# use rullst::realtime::{RealtimeSse, RealtimeSseConfig};
async fn serve(app: rullst::Router, sse: RealtimeSse) -> Result<(), rullst::ServerError> {
    let shutdown = async move {
        let _ = rullst::async_runtime::tokio::signal::ctrl_c().await;
        sse.shutdown();
    };
    rullst::Server::new(app).run_with_shutdown(3000, shutdown).await
}
```

After `shutdown()` open streams end and their browsers reconnect after the
reconnect delay (below). A request that arrives while the server drains, or
beyond the connection limit, opens no channel: it receives a stream that
holds only a comment naming the reason and a `retry:` of a random 5–10
seconds, and ends. `EventSource` waits that long before it tries again, and
the random part keeps browsers refused together from returning together. An
error status such as `503 Service Unavailable` would not slow them down:
`EventSource` ignores `Retry-After` and gives up on an error status, and the
htmx extension then retries on its own schedule, the first time within half a
second.

---

## Limits

`RealtimeSseConfig` bounds every connection; each setter clamps its value:

| Setting | Default | Range | Effect |
| :--- | :--- | :--- | :--- |
| `with_keep_alive` | 15 s | 1 s – 5 min | A `: keep-alive` comment on an idle stream, so proxies keep it open. |
| `with_reconnect_delay` | 1 s | 100 ms – 5 min | Sent first on every stream as its `retry:` field: how long the browser waits before reconnecting once the stream ends (about 3 s without it). |
| `with_max_lifetime` | 30 min | 1 s – 24 h | The stream then ends and the browser reconnects after the reconnect delay, so the route checks the session again. |
| `with_max_connections` | 1,024 | 1 – 65,536 | Open streams across every clone; a request beyond it receives only a `retry:` of 5–10 s (once to twice the reconnect delay, if longer) and ends. |
| `with_max_payload_bytes` | 64 KiB | 1 B – 64 KiB | A larger message is not sent to the connection. |

When a stream reaches its lifetime it ends cleanly and the browser
reconnects after the reconnect delay, so with the defaults a dashboard misses
about one second of updates every 30 minutes and the next fragment corrects
it. htmx reports each such reconnect with an `htmx:sseError` event, which it
also logs to the browser console, followed by `htmx:sseOpen`.

A connection that falls behind its channel's queue (100 messages for a
`BroadcastManager` channel) is ended rather than buffered without limit; the
browser reconnects after the reconnect delay and receives the next fragment. Event names must be 1–128
bytes of ASCII letters, digits, `-`, `_`, `.`, `:` or `/`; other messages are
skipped. Responses carry `Cache-Control: no-cache` and
`X-Accel-Buffering: no`, so nginx forwards each event at once.

For one stream per tenant, subscribe through `TenantRealtime` (see
[Tutorial 17](17-websockets-realtime-chat.md)) and pass the receiver to
`RealtimeSse::subscribe`; `TenantRealtime::publish` already limits payloads to
64 KiB.

---

## Key takeaways

- The server renders HTML with `html!`; the browser only swaps it in. There is
  no client-side state to keep in sync.
- Delivery is process-local and best effort, like `BroadcastManager`: with
  several application instances, each one publishes to its own browsers, so
  feed every instance from the shared source (database, queue or messaging).
- The stream route needs the same authentication and authorization as the
  page; `RealtimeSse` does not know who is connected.

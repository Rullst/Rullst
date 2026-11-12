//! The server: the telemetry receiver, the device registry, the dashboard
//! and Nexus.

use std::sync::Arc;

use crate::registry::{CreateIotDevicesTable, IotDevice, SqlDeviceDirectory};
use rullst::db::schema::Migration;
use rullst::htmx::{HtmxRequest, render_page_with_head};
use rullst::iot_ingest::{TelemetryBatch, TelemetryReceiver, TelemetryReceiverConfig};
use rullst::realtime::{BroadcastManager, RealtimeSse, RealtimeSseConfig};
use rullst::response::{Html, Response};
use rullst::security::{CspNonce, MachineEndpointPolicy};
use rullst::web::axum::{self, Extension, extract::State, routing::get};
use rullst::{Router, html};
use tokio::sync::broadcast::error::RecvError;

use crate::dashboard::{DASHBOARD_CHANNEL, Dashboard};

/// Where gateways post telemetry.
pub const TELEMETRY_PATH: &str = "/iot/telemetry";

/// The example's migrations: only the registry table.
pub fn migrations() -> Vec<Box<dyn Migration>> {
    vec![Box::new(CreateIotDevicesTable)]
}

/// The assembled application.
pub struct App {
    pub router: Router,
    /// Registers the telemetry route as the only machine endpoint.
    pub policy: MachineEndpointPolicy,
    /// Shut it down with the server so open dashboards reconnect elsewhere.
    pub sse: RealtimeSse,
    pub dashboard: Arc<Dashboard>,
}

/// Builds the application; call it inside the Tokio runtime (it starts the
/// task that turns accepted telemetry into dashboard updates).
pub fn app(nexus: Option<axum::Router>) -> Result<App, Box<dyn std::error::Error>> {
    let channels = Arc::new(BroadcastManager::new());
    let config = TelemetryReceiverConfig::new();
    let receiver = TelemetryReceiver::new(SqlDeviceDirectory::new(), Arc::clone(&channels), config);
    let policy = MachineEndpointPolicy::new(vec![receiver.machine_endpoint(TELEMETRY_PATH)?])?;
    let sse = RealtimeSse::new(RealtimeSseConfig::new());
    let dashboard = Arc::new(Dashboard::new(Arc::clone(&channels), sse.clone()));

    // Subscribe before the first request can publish.
    let mut accepted = channels
        .get_or_create(receiver.config().channel())
        .subscribe();
    let updates = Arc::clone(&dashboard);
    rullst::runtime::spawn(async move {
        loop {
            match accepted.recv().await {
                Ok(message) => {
                    if let Ok(batch) = serde_json::from_str::<TelemetryBatch>(&message.payload) {
                        updates.apply(&batch);
                        updates.publish().await;
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });

    let pages = axum::Router::new()
        .route("/", get(page))
        .route("/events", get(events))
        .with_state(Arc::clone(&dashboard));
    let router = receiver
        .routes(TELEMETRY_PATH)
        .merge_axum(pages)
        .merge_axum(rullst::health::health_router());
    let router = match nexus {
        Some(nexus) => router.nest_axum("/nexus", nexus),
        None => router,
    };
    Ok(App {
        router,
        policy,
        sse,
        dashboard,
    })
}

/// Nexus with the registry model, or `None` (with the reason printed) when
/// a release build has no admin credentials.
pub fn nexus() -> Result<Option<axum::Router>, Box<dyn std::error::Error>> {
    match rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env() {
        Ok(auth) => Ok(Some(
            rullst::nexus::Nexus::new()
                .with_auth_policy(auth)
                .with_brand("IoT gateway admin")
                .register::<IotDevice>()
                .try_build()?,
        )),
        Err(rullst::nexus::NexusBuildError::MissingCredential { variable }) => {
            crate::note(format_args!(
                "Nexus is not mounted at /nexus: {variable} is not set"
            ));
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

async fn page(
    State(dashboard): State<Arc<Dashboard>>,
    htmx: HtmxRequest,
    nonce: Option<Extension<CspNonce>>,
) -> Html<String> {
    let devices = SqlDeviceDirectory::new().list().await.unwrap_or_default();
    let (cards, history) = dashboard.fragments(&devices);
    let nonce = nonce.map(|Extension(nonce)| nonce.as_str().to_owned());
    let head = html! {
        <script nonce?={nonce} src="/static/htmx-ext-sse-2.2.4.js"></script>
    };
    let content = html! {
        <main class="wrap" hx-ext="sse" sse-connect="/events">
            <h1>IoT gateway</h1>
            <p class="muted">
                "Each board's gateway posts its readings with that board's own token. "
                "Revoke a device and its readings stop at once."
            </p>
            <section id="devices" sse-swap="devices" aria-live="polite">{cards}</section>
            <section id="history" sse-swap="history">{history}</section>
        </main>
    };
    render_page_with_head(&htmx, "en", "IoT gateway", head, content)
}

async fn events(State(dashboard): State<Arc<Dashboard>>) -> Response {
    dashboard
        .sse
        .channel(&dashboard.channels, DASHBOARD_CHANNEL)
}

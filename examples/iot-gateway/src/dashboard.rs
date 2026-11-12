//! The live dashboard: one card per registered device and the latest
//! batches, pushed over Server-Sent Events.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::registry::{IotDevice, SqlDeviceDirectory};
use rullst::html;
use rullst::html::Markup;
use rullst::iot_ingest::TelemetryBatch;
use rullst::realtime::{BroadcastManager, RealtimeSse};

/// The channel the dashboard page listens to.
pub const DASHBOARD_CHANNEL: &str = "iot.dashboard";
/// Batches kept for the history table.
pub const HISTORY_LEN: usize = 20;

/// The latest values of one device.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Live {
    pub metrics: BTreeMap<String, f64>,
    pub received_at: u64,
    pub batches: u64,
}

/// One row of the history table.
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryRow {
    pub device_id: String,
    pub received_at: u64,
    pub metrics: BTreeMap<String, f64>,
}

#[derive(Default)]
struct State {
    live: BTreeMap<String, Live>,
    history: VecDeque<HistoryRow>,
}

/// Shared by the receiver's subscriber task and the page routes.
pub struct Dashboard {
    pub channels: Arc<BroadcastManager>,
    pub sse: RealtimeSse,
    state: Mutex<State>,
}

impl Dashboard {
    pub fn new(channels: Arc<BroadcastManager>, sse: RealtimeSse) -> Self {
        Self {
            channels,
            sse,
            state: Mutex::new(State::default()),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Folds one accepted batch into the live state.
    pub fn apply(&self, batch: &TelemetryBatch) {
        let metrics: BTreeMap<String, f64> = batch
            .readings
            .iter()
            .map(|reading| (reading.metric.clone(), reading.value))
            .collect();
        let mut state = self.state();
        let live = state.live.entry(batch.device_id.clone()).or_default();
        live.metrics.extend(metrics.clone());
        live.received_at = batch.received_at;
        live.batches += 1;
        state.history.push_front(HistoryRow {
            device_id: batch.device_id.clone(),
            received_at: batch.received_at,
            metrics,
        });
        state.history.truncate(HISTORY_LEN);
    }

    /// The device cards for `devices` (the registry) and the history table.
    pub fn fragments(&self, devices: &[IotDevice]) -> (Markup, Markup) {
        let state = self.state();
        (
            devices_view(devices, &state.live),
            history_view(&state.history),
        )
    }

    /// Re-renders both fragments and pushes them to every open page.
    pub async fn publish(&self) {
        let devices = SqlDeviceDirectory::new().list().await.unwrap_or_default();
        let (devices, history) = self.fragments(&devices);
        let _ = self
            .channels
            .publish(DASHBOARD_CHANNEL, "devices", devices.as_str());
        let _ = self
            .channels
            .publish(DASHBOARD_CHANNEL, "history", history.as_str());
    }
}

fn fixed(value: Option<&f64>, digits: usize) -> String {
    match value {
        Some(value) if value.is_finite() => format!("{value:.digits$}"),
        _ => "-".to_owned(),
    }
}

fn status(device: &IotDevice) -> (&'static str, &'static str) {
    if device.is_revoked() {
        ("revoked", "badge badge-revoked")
    } else if device.last_seen_at.is_some() {
        ("active", "badge badge-active")
    } else {
        ("never seen", "badge")
    }
}

pub fn devices_view(devices: &[IotDevice], live: &BTreeMap<String, Live>) -> Markup {
    html! {
        @if devices.is_empty() {
            <p class="muted">No device is registered yet. Run the register command.</p>
        }
        <div class="cards">
            @for device in devices {
                <article class="card" data-device={device.device_id.as_str()}>
                    <header>
                        <h2>{device.name.as_str()}</h2>
                        <span class={status(device).1}>{status(device).0}</span>
                    </header>
                    <p class="muted"><code>{device.device_id.as_str()}</code></p>
                    @if let Some(values) = live.get(&device.device_id) {
                        <dl>
                            <div><dt>"Die temperature (°C, uncalibrated)"</dt>
                                <dd>{fixed(values.metrics.get("temperature_c_uncalibrated"), 1)}</dd></div>
                            <div><dt>Raw ADC</dt>
                                <dd>{fixed(values.metrics.get("temperature_adc_raw"), 0)}</dd></div>
                            <div><dt>Sine</dt>
                                <dd>{fixed(values.metrics.get("sine"), 2)}</dd></div>
                            <div><dt>Sequence</dt>
                                <dd data-seq>{fixed(values.metrics.get("sequence"), 0)}</dd></div>
                        </dl>
                        <p class="muted">
                            {values.batches} batches since the server started, last at
                            {values.received_at}.
                        </p>
                    } @else {
                        <p class="muted">No telemetry since the server started.</p>
                    }
                    <p class="muted">
                        Firmware {device.firmware_version.as_deref().unwrap_or("unknown")}.
                    </p>
                </article>
            }
        </div>
    }
}

pub fn history_view(history: &VecDeque<HistoryRow>) -> Markup {
    html! {
        <table class="history">
            <caption>Last {history.len()} batches, newest first</caption>
            <thead>
                <tr>
                    <th scope="col">Device</th>
                    <th scope="col">Received</th>
                    <th scope="col">Sequence</th>
                    <th scope="col">"Temp °C"</th>
                    <th scope="col">Sine</th>
                </tr>
            </thead>
            <tbody>
                @for row in history {
                    <tr data-device={row.device_id.as_str()}>
                        <td>{row.device_id.as_str()}</td>
                        <td>{row.received_at}</td>
                        <td>{fixed(row.metrics.get("sequence"), 0)}</td>
                        <td>{fixed(row.metrics.get("temperature_c_uncalibrated"), 1)}</td>
                        <td>{fixed(row.metrics.get("sine"), 2)}</td>
                    </tr>
                }
            </tbody>
        </table>
    }
}

//! The bounded telemetry receiver: device authentication through a machine
//! endpoint, validation, per-device rate limits and channel publication.

use super::config::TelemetryReceiverConfig;
use super::directory::{DeviceDirectory, MAX_DEVICE_ID_BYTES};
use super::error::IngestError;
use super::token::{TokenHash, plausible_token};
use rullst_core::realtime::BroadcastManager;
use rullst_core::security::{MachineEndpoint, MachineEndpointError, MachineRequestVerifier};
use rullst_core::web::axum::{
    self, Extension, Json,
    body::Bytes,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use rullst_iot::SensorTelemetry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Header in which a gateway may report the device's firmware version.
pub const FIRMWARE_HEADER: &str = "x-rullst-firmware";
const MAX_FIRMWARE_BYTES: usize = 64;
const MAX_METRIC_BYTES: usize = 64;

/// The identity the receiver's verifier attaches to an authenticated request,
/// as an Axum request extension. Only the verifier creates one.
///
/// Added in v13.0.0-alpha.5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthenticatedDevice {
    device_id: String,
}

impl AuthenticatedDevice {
    /// The device the bearer token belongs to.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }
}

/// What the receiver publishes for each accepted request: a JSON object on
/// [`TelemetryReceiverConfig::channel`], as event
/// [`TelemetryReceiverConfig::event`]. Parse it back with `serde_json`.
///
/// Added in v13.0.0-alpha.5.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TelemetryBatch {
    /// The authenticated device.
    pub device_id: String,
    /// Unix time (seconds) when the receiver accepted the readings.
    pub received_at: u64,
    /// Firmware version from the `X-Rullst-Firmware` header, if sent.
    pub firmware_version: Option<String>,
    /// The readings, in request order.
    pub readings: Vec<SensorTelemetry>,
}

/// The result of an accepted request.
///
/// Added in v13.0.0-alpha.5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct IngestReceipt {
    /// Readings accepted.
    pub accepted: usize,
    /// Channel subscribers the batch reached (0 when nobody listens).
    pub delivered: usize,
}

struct RateWindow {
    started: Instant,
    requests: u32,
}

struct Inner<D> {
    directory: Arc<D>,
    channels: Arc<BroadcastManager>,
    config: TelemetryReceiverConfig,
    windows: Mutex<HashMap<String, RateWindow>>,
}

/// Accepts device telemetry over HTTP and publishes it to a realtime
/// channel.
///
/// Register [`Self::machine_endpoint`] with `Server::with_machine_endpoints`
/// and mount [`Self::routes`] on the same path. The endpoint's verifier
/// requires one `Authorization: Bearer <token>` header whose token belongs
/// to an active device of the [`DeviceDirectory`] (and, like every machine
/// endpoint, refuses requests carrying cookies or browser origin headers);
/// the handler refuses any request that did not pass it, so telemetry is
/// never anonymous. Each accepted request is one JSON [`SensorTelemetry`]
/// object or an array of them, all naming the authenticated device.
///
/// Added in v13.0.0-alpha.5.
pub struct TelemetryReceiver<D> {
    inner: Arc<Inner<D>>,
}

impl<D> Clone for TelemetryReceiver<D> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<D: DeviceDirectory> TelemetryReceiver<D> {
    /// A receiver that authenticates against `directory` and publishes to
    /// `channels`.
    pub fn new(
        directory: D,
        channels: Arc<BroadcastManager>,
        config: TelemetryReceiverConfig,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                directory: Arc::new(directory),
                channels,
                config,
                windows: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// The device directory.
    pub fn directory(&self) -> &D {
        &self.inner.directory
    }

    /// The limits in force.
    pub fn config(&self) -> &TelemetryReceiverConfig {
        &self.inner.config
    }

    /// The `POST path` machine endpoint that authenticates devices; add it
    /// to the application's `MachineEndpointPolicy`.
    pub fn machine_endpoint(
        &self,
        path: impl Into<String>,
    ) -> Result<MachineEndpoint, MachineEndpointError> {
        let verifier = DeviceVerifier {
            directory: Arc::clone(&self.inner.directory),
        };
        MachineEndpoint::verified_bearer(Method::POST, path, verifier)
    }

    /// The `POST path` route that receives telemetry. It answers
    /// `202 Accepted` with `{"accepted": <readings>}` or an [`IngestError`].
    pub fn routes(&self, path: &str) -> rullst_core::Router {
        let routes = axum::Router::new()
            .route(path, post(receive::<D>))
            .with_state(self.clone());
        rullst_core::Router::new().merge_axum(routes)
    }

    /// Validates and publishes one request body for an authenticated device
    /// (the receiver's own route calls this). `headers` supplies the
    /// content type and the optional `X-Rullst-Firmware` header.
    pub async fn accept(
        &self,
        device: &AuthenticatedDevice,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<IngestReceipt, IngestError> {
        self.accept_at(device, headers, body, unix_now(), Instant::now())
            .await
    }

    pub(crate) async fn accept_at(
        &self,
        device: &AuthenticatedDevice,
        headers: &HeaderMap,
        body: &[u8],
        now: u64,
        instant: Instant,
    ) -> Result<IngestReceipt, IngestError> {
        let config = &self.inner.config;
        self.admit(&device.device_id, instant)?;
        if !is_json(headers) {
            return Err(IngestError::UnsupportedMediaType);
        }
        if body.len() > config.max_body_bytes {
            return Err(IngestError::PayloadTooLarge);
        }
        let firmware = firmware_version(headers)?;
        let readings = match serde_json::from_slice::<Body>(body) {
            Ok(Body::One(reading)) => vec![reading],
            Ok(Body::Many(readings)) => readings,
            Err(_) => return Err(IngestError::Malformed),
        };
        if readings.is_empty() {
            return Err(IngestError::Malformed);
        }
        if readings.len() > config.max_readings {
            return Err(IngestError::TooManyReadings);
        }
        for (index, reading) in readings.iter().enumerate() {
            if reading.device_id != device.device_id {
                return Err(IngestError::DeviceMismatch { index });
            }
            if let Some(reason) = invalid_reading(reading, now, config) {
                return Err(IngestError::InvalidReading { index, reason });
            }
        }
        self.inner
            .directory
            .record_seen(&device.device_id, now, firmware.as_deref())
            .await
            .map_err(|_| IngestError::Unavailable)?;
        let accepted = readings.len();
        let batch = TelemetryBatch {
            device_id: device.device_id.clone(),
            received_at: now,
            firmware_version: firmware,
            readings,
        };
        let delivered = serde_json::to_string(&batch)
            .ok()
            .and_then(|payload| {
                self.inner
                    .channels
                    .publish(&config.channel, &config.event, &payload)
                    .ok()
            })
            .unwrap_or(0);
        Ok(IngestReceipt {
            accepted,
            delivered,
        })
    }

    /// Counts one request of `device_id` in its fixed window.
    fn admit(&self, device_id: &str, now: Instant) -> Result<(), IngestError> {
        let config = &self.inner.config;
        let mut windows = self
            .inner
            .windows
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if !windows.contains_key(device_id) && windows.len() >= config.max_tracked_devices {
            windows.retain(|_, window| now.duration_since(window.started) < config.window);
            if windows.len() >= config.max_tracked_devices {
                return Err(IngestError::RateLimited {
                    retry_after: config.window,
                });
            }
        }
        let window = windows.entry(device_id.to_owned()).or_insert(RateWindow {
            started: now,
            requests: 0,
        });
        let elapsed = now.duration_since(window.started);
        if elapsed >= config.window {
            *window = RateWindow {
                started: now,
                requests: 0,
            };
        }
        if window.requests >= config.requests_per_window {
            return Err(IngestError::RateLimited {
                retry_after: config.window.saturating_sub(elapsed),
            });
        }
        window.requests += 1;
        Ok(())
    }
}

/// One reading or a batch.
#[derive(Deserialize)]
#[serde(untagged)]
enum Body {
    One(SensorTelemetry),
    Many(Vec<SensorTelemetry>),
}

fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
}

fn firmware_version(headers: &HeaderMap) -> Result<Option<String>, IngestError> {
    let mut values = headers.get_all(FIRMWARE_HEADER).iter();
    let Some(value) = values.next() else {
        return Ok(None);
    };
    let version = value.to_str().map_err(|_| IngestError::InvalidFirmware)?;
    if values.next().is_some()
        || !(1..=MAX_FIRMWARE_BYTES).contains(&version.len())
        || !version.bytes().all(|b| b.is_ascii_graphic())
    {
        return Err(IngestError::InvalidFirmware);
    }
    Ok(Some(version.to_owned()))
}

/// Why `reading` cannot be accepted at `now`, if it cannot.
fn invalid_reading(
    reading: &SensorTelemetry,
    now: u64,
    config: &TelemetryReceiverConfig,
) -> Option<&'static str> {
    let metric_ok = (1..=MAX_METRIC_BYTES).contains(&reading.metric.len())
        && reading
            .metric
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if !metric_ok {
        return Some("metric must be 1-64 bytes of ASCII letters, digits, '.', '_' or '-'");
    }
    if !reading.value.is_finite() {
        return Some("value must be a finite number");
    }
    if reading.timestamp > now.saturating_add(config.max_clock_skew.as_secs()) {
        return Some("timestamp is in the future");
    }
    if reading.timestamp < now.saturating_sub(config.max_reading_age.as_secs()) {
        return Some("timestamp is too old");
    }
    None
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

async fn receive<D: DeviceDirectory>(
    State(receiver): State<TelemetryReceiver<D>>,
    device: Option<Extension<AuthenticatedDevice>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(Extension(device)) = device else {
        return IngestError::Unauthenticated.into_response();
    };
    match receiver.accept(&device, &headers, &body).await {
        Ok(receipt) => {
            let mut response = (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({ "accepted": receipt.accepted })),
            )
                .into_response();
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
        Err(error) => error.into_response(),
    }
}

/// Authenticates a device bearer token against the directory.
struct DeviceVerifier<D> {
    directory: Arc<D>,
}

#[async_trait::async_trait]
impl<D: DeviceDirectory> MachineRequestVerifier for DeviceVerifier<D> {
    async fn verify(&self, mut request: Request) -> Result<Request, MachineEndpointError> {
        let hash = {
            let mut values = request.headers().get_all(header::AUTHORIZATION).iter();
            let token = values
                .next()
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .filter(|token| plausible_token(token))
                .ok_or(MachineEndpointError::Unauthorized)?;
            if values.next().is_some() {
                return Err(MachineEndpointError::Unauthorized);
            }
            TokenHash::of(token)
        };
        let device = self
            .directory
            .find_active(&hash)
            .await
            .map_err(|_| MachineEndpointError::Unauthorized)?
            .filter(|device| (1..=MAX_DEVICE_ID_BYTES).contains(&device.device_id.len()))
            .ok_or(MachineEndpointError::Unauthorized)?;
        request.extensions_mut().insert(AuthenticatedDevice {
            device_id: device.device_id,
        });
        Ok(request)
    }
}

#[cfg(test)]
impl<D> TelemetryReceiver<D> {
    pub(crate) fn inner_channels(&self) -> &BroadcastManager {
        &self.inner.channels
    }
}

#[cfg(test)]
impl AuthenticatedDevice {
    pub(crate) fn for_tests(device_id: &str) -> Self {
        Self {
            device_id: device_id.to_owned(),
        }
    }
}

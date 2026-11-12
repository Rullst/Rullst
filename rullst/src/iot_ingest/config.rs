//! Limits of the telemetry receiver.

use std::time::Duration;

/// Limits of a [`super::TelemetryReceiver`]. Every setter clamps its value, so a
/// configuration is always valid.
///
/// Added in v13.0.0-alpha.5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelemetryReceiverConfig {
    pub(super) channel: String,
    pub(super) event: String,
    pub(super) max_body_bytes: usize,
    pub(super) max_readings: usize,
    pub(super) requests_per_window: u32,
    pub(super) window: Duration,
    pub(super) max_clock_skew: Duration,
    pub(super) max_reading_age: Duration,
    pub(super) max_tracked_devices: usize,
}

impl Default for TelemetryReceiverConfig {
    fn default() -> Self {
        Self {
            channel: "iot.telemetry".to_owned(),
            event: "telemetry".to_owned(),
            max_body_bytes: 16 * 1024,
            max_readings: 64,
            requests_per_window: 120,
            window: Duration::from_secs(60),
            max_clock_skew: Duration::from_secs(300),
            max_reading_age: Duration::from_secs(24 * 60 * 60),
            max_tracked_devices: 10_000,
        }
    }
}

impl TelemetryReceiverConfig {
    /// The defaults: channel `iot.telemetry`, event `telemetry`, 16 KiB
    /// bodies, 64 readings per request, 120 requests per device per minute,
    /// timestamps at most 5 minutes ahead and 24 hours old, and rate state
    /// for at most 10,000 devices.
    pub fn new() -> Self {
        Self::default()
    }

    /// The `BroadcastManager` channel accepted batches are published to.
    /// A name that is not 1–128 bytes of ASCII letters, digits, `-`, `_`,
    /// `.`, `:` or `/` is ignored and the current one kept.
    pub fn with_channel(mut self, channel: impl Into<String>) -> Self {
        let channel = channel.into();
        if realtime_name(&channel) {
            self.channel = channel;
        }
        self
    }

    /// The event name of published batches, with the same rules as
    /// [`Self::with_channel`].
    pub fn with_event(mut self, event: impl Into<String>) -> Self {
        let event = event.into();
        if realtime_name(&event) {
            self.event = event;
        }
        self
    }

    /// Largest request body, clamped to 256 bytes–1 MiB (the machine
    /// endpoint buffers at most 1 MiB).
    pub fn with_max_body_bytes(mut self, bytes: usize) -> Self {
        self.max_body_bytes = bytes.clamp(256, 1024 * 1024);
        self
    }

    /// Most readings in one request, clamped to 1–1,024.
    pub fn with_max_readings(mut self, readings: usize) -> Self {
        self.max_readings = readings.clamp(1, 1024);
        self
    }

    /// Requests one device may send per `window`, clamped to 1–100,000
    /// requests and a 1-second–1-hour window. Beyond it the device receives
    /// `429 Too Many Requests` with `Retry-After`.
    pub fn with_rate_limit(mut self, requests: u32, window: Duration) -> Self {
        self.requests_per_window = requests.clamp(1, 100_000);
        self.window = window.clamp(Duration::from_secs(1), Duration::from_secs(3600));
        self
    }

    /// How far a reading's timestamp may be ahead of the server clock and
    /// how old it may be, clamped to 0–1 hour and 1 minute–30 days.
    pub fn with_timestamp_window(mut self, max_skew: Duration, max_age: Duration) -> Self {
        self.max_clock_skew = max_skew.min(Duration::from_secs(3600));
        self.max_reading_age = max_age.clamp(
            Duration::from_secs(60),
            Duration::from_secs(30 * 24 * 60 * 60),
        );
        self
    }

    /// The channel batches are published to.
    pub fn channel(&self) -> &str {
        &self.channel
    }

    /// The event name of published batches.
    pub fn event(&self) -> &str {
        &self.event
    }

    /// Largest accepted body, in bytes.
    pub fn max_body_bytes(&self) -> usize {
        self.max_body_bytes
    }

    /// Most readings per request.
    pub fn max_readings(&self) -> usize {
        self.max_readings
    }

    /// Requests per device per [`Self::window`].
    pub fn requests_per_window(&self) -> u32 {
        self.requests_per_window
    }

    /// The rate-limit window.
    pub fn window(&self) -> Duration {
        self.window
    }
}

fn realtime_name(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/'))
}

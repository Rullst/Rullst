//! MQTT-shaped formatting fixture for tests and demos.
//!
//! This module is available only with `experimental-simulators`. It does not
//! encode MQTT packets, connect to a broker, publish, subscribe, or implement
//! any MQTT protocol version.

extern crate alloc;

use crate::SensorTelemetry;
use alloc::string::{String, ToString};

/// Explicitly simulated telemetry value formatter; not an MQTT client.
pub struct SimulatedMqttPayloadFormatter;

impl SimulatedMqttPayloadFormatter {
    /// Formats only the numeric reading for deterministic fixture assertions.
    #[must_use]
    pub fn format_value(telemetry: &SensorTelemetry) -> String {
        telemetry.value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulated_formatter_only_formats_a_value() {
        let telemetry = SensorTelemetry::new("fixture", "temperature", 24.5, 1);
        assert_eq!(
            SimulatedMqttPayloadFormatter::format_value(&telemetry),
            "24.5"
        );
    }
}

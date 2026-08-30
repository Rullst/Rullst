//! In-memory digital-twin state and JSON serialization helpers.

extern crate alloc;
use crate::SensorTelemetry;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use serde::{Deserialize, Serialize};

/// Failure to serialize a Digital Twin snapshot without losing sensor data.
#[derive(Debug)]
pub enum TwinSyncError {
    /// JSON has no representation for NaN or infinity.
    NonFiniteReading,
    /// Serialization failed for another reason.
    Serialization(serde_json::Error),
}

impl fmt::Display for TwinSyncError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteReading => formatter.write_str("non-finite sensor reading"),
            Self::Serialization(error) => {
                write!(formatter, "snapshot serialization failed: {error}")
            }
        }
    }
}

impl From<serde_json::Error> for TwinSyncError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

/// A local snapshot representing the reported state of an IoT device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DigitalTwin {
    pub device_id: String,
    pub readings: Vec<SensorTelemetry>,
    pub is_online: bool,
}

impl DigitalTwin {
    /// Creates a new Digital Twin for a device.
    pub fn new(device_id: impl Into<String>) -> Self {
        Self {
            device_id: device_id.into(),
            readings: Vec::new(),
            is_online: true,
        }
    }

    /// Ingests a new sensor reading into the twin's state.
    pub fn ingest(&mut self, telemetry: SensorTelemetry) {
        self.readings.push(telemetry);
    }

    /// Returns the latest reading for a given metric.
    pub fn latest(&self, metric: &str) -> Option<&SensorTelemetry> {
        self.readings.iter().rev().find(|t| t.metric == metric)
    }

    /// Serializes the twin state as a compact JSON string.
    pub fn try_to_sync_payload(&self) -> Result<String, TwinSyncError> {
        if self
            .readings
            .iter()
            .any(|reading| !reading.value.is_finite())
        {
            return Err(TwinSyncError::NonFiniteReading);
        }
        serde_json::to_string(self).map_err(TwinSyncError::from)
    }

    /// Serializes the twin state, returning a static valid JSON error payload
    /// if a non-finite reading cannot be represented by `serde_json`.
    ///
    /// This is a local snapshot helper; it does not perform network sync.
    pub fn to_sync_payload(&self) -> String {
        self.try_to_sync_payload()
            .unwrap_or_else(|_| String::from("{\"error\":\"serialize_failed\"}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digital_twin_ingest_and_latest() {
        let mut twin = DigitalTwin::new("esp32-farm-01");
        twin.ingest(SensorTelemetry::new(
            "esp32-farm-01",
            "temperature",
            22.5,
            1700000001,
        ));
        twin.ingest(SensorTelemetry::new(
            "esp32-farm-01",
            "temperature",
            23.1,
            1700000002,
        ));
        twin.ingest(SensorTelemetry::new(
            "esp32-farm-01",
            "humidity",
            55.0,
            1700000003,
        ));

        let latest_temp = twin.latest("temperature").unwrap();
        assert_eq!(latest_temp.value, 23.1);

        let latest_humidity = twin.latest("humidity").unwrap();
        assert_eq!(latest_humidity.value, 55.0);
    }

    #[test]
    fn test_digital_twin_sync_payload() {
        let mut twin = DigitalTwin::new("esp32-gateway");
        twin.ingest(SensorTelemetry::new(
            "esp32-gateway",
            "vibration",
            1.3,
            1700000000,
        ));
        let payload = twin.to_sync_payload();
        assert!(payload.contains("esp32-gateway"));
        assert_eq!(twin.try_to_sync_payload().expect("valid payload"), payload);
    }

    #[test]
    fn invalid_float_fallback_is_valid_json_and_cannot_inject_device_id() {
        let mut twin = DigitalTwin::new("\"},\"injected\":true,{\"x\":\"");
        twin.ingest(SensorTelemetry::new("device", "temperature", f64::NAN, 1));
        assert!(twin.try_to_sync_payload().is_err());

        let payload = twin.to_sync_payload();
        assert_eq!(payload, "{\"error\":\"serialize_failed\"}");
        let parsed: serde_json::Value =
            serde_json::from_str(&payload).expect("fallback must be valid JSON");
        assert_eq!(parsed["error"], "serialize_failed");
        assert!(parsed.get("injected").is_none());
    }
}

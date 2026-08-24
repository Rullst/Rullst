//! In-memory digital-twin state and JSON serialization helpers.

extern crate alloc;
use crate::SensorTelemetry;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

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

    /// Serializes the twin state as a compact JSON string for cloud sync.
    pub fn to_sync_payload(&self) -> String {
        if let Ok(json) = serde_json::to_string(self) {
            json
        } else {
            alloc::format!(
                "{{\"device_id\":\"{}\",\"error\":\"serialize_failed\"}}",
                self.device_id
            )
        }
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
    }
}

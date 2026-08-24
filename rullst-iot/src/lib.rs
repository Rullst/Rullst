//! Rullst IoT data models, protocol frame helpers, and signed firmware gate.
//!
//! This crate does not currently provide MQTT network transport, hardware HSM
//! bindings, or post-quantum cryptography. Deterministic fixtures resembling
//! those capabilities require the explicit `experimental-simulators` feature.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::string::String;
use serde::{Deserialize, Serialize};

// Explicitly simulated protocol fixtures.
#[cfg(feature = "experimental-simulators")]
pub mod mqtt;

// State models and protocol frame helpers (not hardware drivers).
pub mod ble;
pub mod gpio;
pub mod i2c;
pub mod modbus;

// Statistical evaluation and HTML rendering helpers.
pub mod anomaly;
pub mod ui;

// Topology state and signed OTA verification.
pub mod mesh;
pub mod ota;

// Explicitly simulated cryptographic fixtures.
#[cfg(feature = "experimental-simulators")]
pub mod hsm;
#[cfg(feature = "experimental-simulators")]
pub mod pqc;

// Power recommendations and digital-twin state.
pub mod power;
pub mod twin;

// Re-exports
pub use anomaly::{AnomalyDetector, AnomalyState};
pub use ble::{GattCharacteristic, GattService};
pub use gpio::{GpioPin, PinMode, PinState};
#[cfg(feature = "experimental-simulators")]
pub use hsm::{SimulatedHsmDevice, SimulatedHsmProfile};
pub use i2c::I2cHelper;
pub use mesh::{MeshNode, MeshTopology, NodeStatus};
pub use modbus::{ModbusFrame, ModbusFunction};
#[cfg(feature = "experimental-simulators")]
pub use mqtt::SimulatedMqttPayloadFormatter;
pub use ota::{BootPartition, OtaCommit, OtaError, OtaManager, OtaManifest, OtaStatus};
pub use power::{HarvesterState, PowerGovernor, PowerMode};
#[cfg(feature = "experimental-simulators")]
pub use pqc::SimulatedPqcFixture;
pub use twin::DigitalTwin;
pub use ui::IotDashboard;

/// Sensor payload telemetry model for IoT nodes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorTelemetry {
    /// Device identifier string.
    pub device_id: String,
    /// Sensor metric name (e.g. "temperature", "humidity", "vibration").
    pub metric: String,
    /// Floating point metric reading.
    pub value: f64,
    /// UNIX timestamp in seconds.
    pub timestamp: u64,
}

impl SensorTelemetry {
    /// Creates a new SensorTelemetry instance.
    pub fn new(
        device_id: impl Into<String>,
        metric: impl Into<String>,
        value: f64,
        timestamp: u64,
    ) -> Self {
        Self {
            device_id: device_id.into(),
            metric: metric.into(),
            value,
            timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_telemetry() {
        let t = SensorTelemetry::new("esp32-node-01", "temperature", 24.5, 1700000000);
        assert_eq!(t.device_id, "esp32-node-01");
        assert_eq!(t.metric, "temperature");
        assert_eq!(t.value, 24.5);
    }
}

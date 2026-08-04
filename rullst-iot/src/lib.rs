//! Rullst IoT Core & Embedded Hardware Telemetry Suite.
//!
//! ## Hardware Compatibility
//! | Target | Mode | RAM Footprint |
//! |--------|------|--------------|
//! | Raspberry Pi / Orange Pi | `std` | < 2MB |
//! | ESP32 (S3/C3) | `no_std` | < 256KB |
//! | STM32 (Cortex-M) | `no_std` | < 128KB |
//! | Arduino 32-bit (Due, Nano BLE) | `no_std` | < 64KB |

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::string::{String, ToString};
use serde::{Deserialize, Serialize};

// Phase 1: Core Telemetry & Protocols
pub mod mqtt;

// Phase 2: Hardware HAL & Protocols
pub mod gpio;
pub mod i2c;
pub mod modbus;
pub mod ble;

// Phase 3: Edge AI & Micro-UI
pub mod anomaly;
pub mod ui;

// Phase 4: Autonomous Swarm & OTA
pub mod mesh;
pub mod ota;

// Phase 5: Hardware Security & PQC
pub mod hsm;
pub mod pqc;

// Phase 6: Power & Digital Twin
pub mod power;
pub mod twin;

// Re-exports
pub use gpio::{GpioPin, PinMode, PinState};
pub use i2c::I2cHelper;
pub use modbus::{ModbusFrame, ModbusFunction};
pub use ble::{GattService, GattCharacteristic};
pub use anomaly::{AnomalyDetector, AnomalyState};
pub use ui::IotDashboard;
pub use mesh::{MeshTopology, MeshNode, NodeStatus};
pub use ota::{OtaManager, BootPartition, OtaStatus};
pub use hsm::{HsmDevice, HsmChipType};
pub use pqc::PqcKeyPair;
pub use power::{PowerGovernor, PowerMode, HarvesterState};
pub use twin::DigitalTwin;

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
    pub fn new(device_id: impl Into<String>, metric: impl Into<String>, value: f64, timestamp: u64) -> Self {
        Self {
            device_id: device_id.into(),
            metric: metric.into(),
            value,
            timestamp,
        }
    }
}

/// Lightweight MQTT / CoAP protocol driver helper.
pub struct MqttDriver;

impl MqttDriver {
    /// Formats a telemetry model into an MQTT topic payload string.
    pub fn format_mqtt_payload(telemetry: &SensorTelemetry) -> String {
        telemetry.value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_telemetry() {
        let t = SensorTelemetry::new("esp32-node-01", "temperature", 24.5, 1700000000);
        assert_eq!(t.device_id, "esp32-node-01");
        let payload = MqttDriver::format_mqtt_payload(&t);
        assert_eq!(payload, "24.5");
    }
}

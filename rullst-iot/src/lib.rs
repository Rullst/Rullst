//! Rullst IoT data models, protocol frame helpers, and signed firmware gate.
//!
//! This crate does not currently provide MQTT/CoAP network transport, hardware
//! HSM bindings, or post-quantum cryptography. It does provide bounded MQTT 5
//! PUBLISH and CoAP request encoders. Deterministic fixtures resembling other
//! capabilities require the explicit `experimental-simulators` feature.
//!
//! **Maturity: Extension**
//! ([tiers](https://rullst.github.io/Rullst/book/maturity.html)) since
//! 13.0.0-alpha.5: optional, and supported for its documented scope. On
//! 2026-11-10 a BBC micro:bit V2 (nRF52833) ran the encoders, the OTA gate
//! (also streamed and with `cargo rullst iot:sign` output) and the
//! counter-store contract, and on 2026-11-11 two Arduino Uno boards sent
//! telemetry through a Rullst gateway with no broker. Network transports, HSM,
//! flashing and bootloaders are the application's. The default `std` feature can be
//! disabled for `no_std` targets, which need `alloc` with a
//! `#[global_allocator]` and about 8 KiB of stack for the Ed25519 check in
//! [`OtaManager::begin_update`]; its [`OtaImageVerifier`] hashes an image in
//! chunks, so the image never has to be one slice in memory. Start with
//! [`OtaManager`] for the signed
//! firmware gate, or [`MqttPublish`] and [`CoapRequest`] for the packet
//! encoders. The crate guide's [module
//! status](https://rullst.github.io/Rullst/book/crates/iot.html#module-status)
//! table says what each module implements.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;
use alloc::string::String;
use serde::{Deserialize, Serialize};

// Bounded protocol encoders (not network clients or transports).
pub mod coap;
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
pub use coap::{CoapCodecError, CoapMessageType, CoapMethod, CoapRequest, MAX_COAP_DATAGRAM_BYTES};
pub use gpio::{GpioPin, PinMode, PinState};
#[cfg(feature = "experimental-simulators")]
pub use hsm::{SimulatedHsmDevice, SimulatedHsmProfile};
pub use i2c::{I2cFrameError, I2cHelper, MAX_I2C_READ_BYTES};
pub use mesh::{MeshNode, MeshTopology, NodeStatus};
pub use modbus::{ModbusFrame, ModbusFunction};
#[cfg(feature = "experimental-simulators")]
pub use mqtt::SimulatedMqttPayloadFormatter;
pub use mqtt::{MAX_MQTT_PACKET_BYTES, MqttCodecError, MqttPublish, MqttQos};
pub use ota::{
    BootPartition, OtaCommit, OtaError, OtaImageVerifier, OtaManager, OtaManifest, OtaStatus,
    RollbackCounterError, RollbackCounterStore,
};
pub use power::{HarvesterState, PowerGovernor, PowerMode};
#[cfg(feature = "experimental-simulators")]
pub use pqc::SimulatedPqcFixture;
pub use twin::{DigitalTwin, TwinJsonError, TwinSyncError};
pub use ui::{IotDashboard, IotHtml};

/// Sensor payload telemetry model for IoT nodes.
///
/// `timestamp` is whole seconds since the UNIX epoch, the resolution of the
/// real-time clocks of the boards this model targets. A reading that needs
/// finer time carries it in the application's own payload type; this one keeps
/// its field set and JSON shape (`device_id`, `metric`, `value`, `timestamp`)
/// stable for 13.x.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorTelemetry {
    /// Device identifier string.
    pub device_id: String,
    /// Sensor metric name (e.g. "temperature", "humidity", "vibration").
    pub metric: String,
    /// Floating point metric reading.
    pub value: f64,
    /// UNIX timestamp in whole seconds (not milliseconds).
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

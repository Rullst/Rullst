//! Authenticated device telemetry ingestion (feature `iot-ingest`).
//!
//! A [`TelemetryReceiver`] accepts `rullst::iot::SensorTelemetry` JSON from
//! devices (or the gateways that speak for them) over HTTP and publishes each
//! accepted request to a `BroadcastManager` channel as a [`TelemetryBatch`],
//! where an SSE dashboard, a queue worker or a database writer picks it up.
//!
//! Every device has its own bearer token ([`DeviceToken`]); a
//! [`DeviceDirectory`] stores only its SHA-256 ([`TokenHash`]) and says which
//! devices are active. The receiver's route is a machine endpoint
//! (`rullst::security::MachineEndpoint::verified_bearer`), so it is exempt
//! from browser CSRF only after the token is verified, and a request with
//! cookies or a browser `Origin` is refused. A revoked token is refused from
//! the next request on. Bodies, readings per request, timestamps and the
//! request rate of each device are bounded, and every refusal is a typed
//! [`IngestError`] with a stable code.
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use rullst::iot_ingest::{MemoryDeviceDirectory, TelemetryReceiver, TelemetryReceiverConfig};
//! use rullst::realtime::BroadcastManager;
//! use rullst::security::MachineEndpointPolicy;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let devices = MemoryDeviceDirectory::new();
//! let token = devices.register("uno-a", "Greenhouse Uno")?;
//! println!("give the gateway this token once: {}", token.expose());
//!
//! let channels = Arc::new(BroadcastManager::new());
//! let receiver = TelemetryReceiver::new(devices, channels, TelemetryReceiverConfig::new());
//! let policy = MachineEndpointPolicy::new(vec![receiver.machine_endpoint("/iot/telemetry")?])?;
//! rullst::Server::new(receiver.routes("/iot/telemetry"))
//!     .with_machine_endpoints(policy)
//!     .run(3000)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! The registry behind a [`DeviceDirectory`] belongs to the application:
//! the repository's `examples/iot-gateway` keeps it in an ORM model with a
//! migration and a Nexus admin page, and [`MemoryDeviceDirectory`] serves
//! tests and fixed device lists.
//!
//! **Maturity: Extension** since 13.0.0-alpha.5, like `rullst-iot`.
//!
//! Added in v13.0.0-alpha.5.

mod config;
mod directory;
mod error;
mod receiver;
mod token;

pub use config::TelemetryReceiverConfig;
pub use directory::{
    DeviceDirectory, DeviceRecord, DirectoryError, MemoryDeviceDirectory, validate_device,
};
pub use error::IngestError;
pub use receiver::{
    AuthenticatedDevice, FIRMWARE_HEADER, IngestReceipt, TelemetryBatch, TelemetryReceiver,
};
pub use token::{DeviceToken, TokenHash};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod bounds_tests;

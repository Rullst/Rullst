# rullst-iot

> **v12 development notice:** This README documents the unreleased v12 source.
> Use a path dependency from this checkout until an immutable v12 RC exists on
> crates.io. `12.0.0-rc.1` below is the planned first RC.

`rullst-iot` provides `no_std`-compatible telemetry models, protocol frame
builders, deterministic edge helpers, and a fail-closed signed firmware gate.

## Implemented scope

- `SensorTelemetry` and `DigitalTwin` in-memory state models.
- Modbus frame/CRC helpers, BLE GATT data structures, I2C frame builders, and
  simulated GPIO state. These are not operating-system or hardware drivers.
- Statistical anomaly evaluation, power policy helpers, and topology models.
- An escaped HTML snapshot-card renderer. It deliberately labels the card as a
  snapshot rather than inferring device connectivity.
- Ed25519 verification of domain-separated OTA manifests. A signed manifest
  binds the device target, version, monotonic rollback counter, firmware length,
  and SHA-256 digest.

The OTA state machine only verifies an artifact and selects the inactive boot
partition. Integrators must still download and flash the image, validate the
written bank, durably persist the rollback counter, and configure/recover the
platform bootloader.

## CLI telemetry scaffold

Inside a Rullst application, the CLI creates and registers a local telemetry
module and enables the umbrella `iot` feature:

```console
cargo rullst make:iot TemperatureSensor
```

The command validates Rust identifiers, refuses path traversal and existing
files, and generates code through `rullst::iot::SensorTelemetry`. It does not
install a HAL, MQTT/CoAP transport, firmware or broker configuration.

## Signed OTA gate

Provision the publisher's Ed25519 public key through a trusted manufacturing or
device-enrollment path. Never obtain that key from the same untrusted update
channel as the firmware.

```rust
use rullst_iot::{OtaManager, OtaManifest};

# fn verify_download(
#     firmware: &[u8],
#     signature: &[u8],
#     trusted_public_key: [u8; 32],
# ) -> Result<(), rullst_iot::OtaError> {
let manifest = OtaManifest::from_firmware(
    "board-revision-a",
    "12.1.0",
    121,
    firmware,
)?;
let mut ota = OtaManager::new_with_trusted_key(
    "board-revision-a",
    "12.0.0",
    120,
    trusted_public_key,
)?;

ota.verify_update(&manifest, firmware, signature)?;
let selection = ota.commit_verified_update()?;

// Platform code may now flash/validate `selection.target_partition()` and must
// persist `selection.rollback_counter()` atomically with its boot decision.
# Ok(())
# }
```

`OtaManager::new`, `verify_signature`, and `commit_update` are deprecated
migration APIs. All three always return `OtaError` because keyless construction,
payload-only verification, and unconditional commit cannot provide the required
guarantees.

## Experimental simulators

The `experimental-simulators` feature exposes deterministic fixtures named
`SimulatedHsmDevice`, `SimulatedPqcFixture`, and
`SimulatedMqttPayloadFormatter`. They are useful only in tests and demos:

```toml
rullst-iot = { version = "12.0.0-rc.1", features = ["experimental-simulators"] }
```

They do **not** provide hardware-backed keys, signatures, ML-KEM/Kyber,
confidentiality, quantum resistance, MQTT packet encoding, or broker transport.
There are intentionally no aliases named `HsmDevice`, `PqcKeyPair`, or
`MqttDriver`.

## Not implemented

- MQTT/CoAP/WebSocket network clients or broker integration.
- ATECC608A, TPM 2.0, or STSAFE hardware backends.
- ML-KEM or any other post-quantum cryptographic primitive.
- Firmware download, delta patching, flash writes, bootloader control, or
  persistent anti-rollback storage.
- Bidirectional Digital Twin transport or Studio/Nexus device synchronization.

See [ROADMAP.md](ROADMAP.md) for the remaining integration work.

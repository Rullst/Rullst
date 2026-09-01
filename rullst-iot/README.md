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
- Bounded `no_std` MQTT 5 PUBLISH and RFC 7252 CoAP request encoders. They
  produce protocol bytes only; the application still owns sockets, TLS/DTLS,
  broker limits, acknowledgements, retries, congestion control, and identity.
- Statistical anomaly evaluation, power policy helpers, and topology models.
- An escaped HTML snapshot-card renderer. It deliberately labels the card as a
  snapshot rather than inferring device connectivity.
- Ed25519 verification of domain-separated OTA manifests. A signed manifest
  binds the device target, version, monotonic rollback counter, firmware length,
  and SHA-256 digest.
- A `no_std` `RollbackCounterStore` boundary for loading durable state and
  atomically committing a strictly increasing counter with compare-and-set.
  Restart, unavailable-store retry, and stale-writer conflicts have executable
  contract tests; device-specific persistence still requires hardware evidence.

The OTA state machine verifies an artifact, exposes the inactive boot partition,
and coordinates with a caller-provided counter store. Integrators must still
download and flash the image, validate the written bank, implement truly durable
counter storage, and configure/recover the platform bootloader.

## CLI telemetry scaffold

Inside a Rullst application, the CLI creates and registers a local telemetry
module and enables the umbrella `iot` feature:

```console
cargo rullst make:iot TemperatureSensor
```

The command validates Rust identifiers, refuses path traversal and existing
files, and generates code through `rullst::iot::SensorTelemetry`. It does not
install a HAL, MQTT/CoAP transport, firmware or broker configuration.

## Bounded protocol encoders

The packet helpers are useful at a transport adapter boundary without pulling a
network runtime into the embedded crate:

```rust
use rullst_iot::{
    CoapMessageType, CoapMethod, CoapRequest, MqttPublish, MqttQos,
};

let mqtt = MqttPublish::reliable(
    "factory/line-1/temperature",
    b"24.5".to_vec(),
    MqttQos::AtLeastOnce,
    7,
)?
.encode()?;

let coap = CoapRequest::new(
    CoapMessageType::Confirmable,
    CoapMethod::Post,
    42,
    [0x01, 0x02],
)?
.path_segment("telemetry")?
.content_format(50)
.payload(br#"{"temperature":24.5}"#.to_vec())?
.encode()?;

# Ok::<(), Box<dyn std::error::Error>>(())
```

`MqttPublish` emits one MQTT 5 PUBLISH packet with an empty property section and
a 1 MiB local ceiling. It does not implement CONNECT, broker negotiation,
PUBACK/PUBREC/PUBREL/PUBCOMP, or retries. `CoapRequest` emits base GET/POST/PUT/
DELETE requests with ordered URI-Path and Content-Format options under a
conservative 1152-byte datagram ceiling; token uniqueness, message correlation,
retransmission, block-wise transfer, UDP and DTLS remain caller responsibilities.

## Signed OTA gate

Provision the publisher's Ed25519 public key through a trusted manufacturing or
device-enrollment path. Never obtain that key from the same untrusted update
channel as the firmware.

```rust
use rullst_iot::{
    OtaCommit, OtaError, OtaManager, OtaManifest, RollbackCounterStore,
};

fn verify_download<S: RollbackCounterStore>(
    firmware: &[u8],
    signature: &[u8],
    trusted_public_key: [u8; 32],
    counter_store: &mut S,
) -> Result<OtaCommit, OtaError> {
let manifest = OtaManifest::from_firmware(
    "board-revision-a",
    "12.1.0",
    121,
    firmware,
)?;
let mut ota = OtaManager::new_with_counter_store(
    "board-revision-a",
    "12.0.0",
    trusted_public_key,
    counter_store,
)?;

ota.verify_update(&manifest, firmware, signature)?;
let target = ota.verified_target_partition()?;

// The platform must flash and read back `target` before this call.
// It must coordinate the returned receipt with its bootloader afterward.
let receipt = ota.commit_verified_update_with_store(counter_store)?;
debug_assert_eq!(receipt.target_partition(), target);

Ok(receipt)
}
```

`RollbackCounterStore::compare_and_set` must make no change on an expected-value
conflict, reject non-increasing values, and return success only after persistence
survives reset. Advancing the counter before a later bootloader failure is
security-safe but can require platform recovery and a newer signed counter; the
framework cannot make counter storage and boot selection one hardware-atomic
operation. `commit_verified_update` remains available for process-local state,
but it does not provide persistent anti-rollback protection.

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
confidentiality, quantum resistance, or broker transport. The MQTT encoder
above is independent of the simulated numeric-value formatter and remains only
a packet helper. There are intentionally no aliases named `HsmDevice`,
`PqcKeyPair`, or `MqttDriver`.

## Not implemented

- MQTT/CoAP/WebSocket network clients or broker integration.
- ATECC608A, TPM 2.0, or STSAFE hardware backends.
- ML-KEM or any other post-quantum cryptographic primitive.
- Firmware download, delta patching, flash writes, bootloader control, or
  a concrete persistent anti-rollback storage implementation.
- Bidirectional Digital Twin transport or Studio/Nexus device synchronization.

See [ROADMAP.md](ROADMAP.md) for the remaining integration work.

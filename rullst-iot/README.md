# rullst-iot

`rullst-iot` provides `no_std`-compatible telemetry models, protocol frame
builders, deterministic edge helpers, and a fail-closed signed firmware gate.

**Maturity: Experimental** ([tiers](https://rullst.github.io/Rullst/book/maturity.html)).
The API may change between 13.x releases, and nothing here has been validated
on physical hardware or against a live broker. Experimental does not mean
broken; the table below shows which modules are working implementations.

## Module status

**Working** means complete for a bounded scope and tested. **Contract** is a
trait your platform implements. **Model only** holds in-memory state and drives
no hardware or network. **Fixture** is deterministic test data behind
`experimental-simulators`, never for production.

| Module | Kind | Scope |
| :--- | :--- | :--- |
| `ota` | Working + Contract | Ed25519 manifest verification and A/B partition selection; `RollbackCounterStore` is the durable-counter contract. No download, flashing, bootloader or concrete store. |
| `mqtt` | Working (encoder) | One MQTT 5 PUBLISH packet. No connection, acknowledgements or TLS. |
| `coap` | Working (encoder) | RFC 7252 base requests. No UDP/DTLS, retransmission or response decoding. |
| `modbus` | Working (narrow) | CRC-16 and a Read Holding Registers RTU request. No response parsing, write builder or transport. |
| `i2c` | Working (narrow) | Register-read transaction bytes with address/length checks. No bus access. |
| `anomaly` | Working | Fixed mean/tolerance classification; non-finite values fail closed. |
| `twin` | Working (in memory) | Readings, latest per metric, JSON snapshot. No sync; readings grow until you trim them. |
| `ui` | Working | Escaped HTML snapshot card. |
| `power` | Working (policy) | Hard-coded voltage thresholds to a recommended mode. Controls no hardware. |
| `mesh` | Model only | Node list and strongest-RSSI relay pick. No routing or transport. |
| `ble` | Model only | GATT data structs. No radio. |
| `gpio` | Model only | In-memory pin state. No register access. |
| `hsm`, `pqc` | Fixture | Deterministic hash bytes. Not an HSM and not post-quantum cryptography. |

## Implemented scope

- `SensorTelemetry` and `DigitalTwin` in-memory state models.
- Modbus frame/CRC helpers, BLE GATT data structures, I2C frame builders, and
  in-memory GPIO state. These are not operating-system or hardware drivers.
  `I2cHelper::try_build_read_frame` rejects reserved or non-7-bit addresses and
  reads above `MAX_I2C_READ_BYTES` (8,192) with a typed `I2cFrameError`;
  `build_read_frame` returns an empty frame in those cases.
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
DELETE requests with ordered URI-Path (at most 255 bytes per segment and never
`.` or `..`, as RFC 7252 requires) and Content-Format options under a
conservative 1152-byte datagram ceiling; token uniqueness, message correlation,
retransmission, block-wise transfer, UDP and DTLS remain caller
responsibilities.

## Signed OTA gate

Provision the publisher's Ed25519 public key through a trusted manufacturing or
device-enrollment path. Never obtain that key from the same untrusted update
channel as the firmware.

```rust
use rullst_iot::{
    BootPartition, OtaCommit, OtaError, OtaManager, OtaManifest, RollbackCounterStore,
};

fn verify_download<S: RollbackCounterStore>(
    firmware: &[u8],
    signature: &[u8],
    trusted_public_key: [u8; 32],
    counter_store: &mut S,
    running_partition: BootPartition,
) -> Result<OtaCommit, OtaError> {
let manifest = OtaManifest::from_firmware(
    "board-revision-a",
    "12.1.0",
    121,
    firmware,
)?;
// `running_partition` is the bank the platform bootloader started.
let mut ota = OtaManager::new_with_running_partition(
    "board-revision-a",
    "12.0.0",
    running_partition,
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
conflict, reject non-increasing values, and return success only after
persistence survives reset. Another error may leave the outcome unknown, such as
a completed write whose acknowledgement was lost. A retry of
`commit_verified_update_with_store` for that same manifest then completes the
commit when the store reports, and a fresh `load` confirms, exactly its
counter. An equal stored counter reached any other way remains a conflict:
a stale manager or a different image carrying the same counter is refused.
Advancing the counter before a later bootloader failure is security-safe but
can require platform recovery and a newer signed counter; the framework cannot
make counter storage and boot selection one hardware-atomic operation.
`commit_verified_update` remains available for process-local state, but it does
not provide persistent anti-rollback protection.

`verified_target_partition` and the receipt always name the bank opposite
`current_partition`. The v13 constructor `new_with_running_partition` takes
the bank the bootloader started. `new_with_counter_store` and
`new_with_trusted_key` assume `PartitionA`, so with them the platform must set
`current_partition` from its bootloader before verifying an update once the
device has rebooted into `PartitionB`.
Committing does not change `current_partition`: the device keeps running the
old bank until the platform reboots, so a further update verified in the same
process targets the same inactive bank, never the running one.

`OtaManager::new`, `verify_signature`, and `commit_update` are deprecated
migration APIs. All three always return `OtaError` because keyless construction,
payload-only verification, and unconditional commit cannot provide the required
guarantees.

## Experimental simulators

The `experimental-simulators` feature exposes deterministic fixtures named
`SimulatedHsmDevice`, `SimulatedPqcFixture`, and
`SimulatedMqttPayloadFormatter`. They are useful only in tests and demos:

```toml
rullst-iot = { version = "12.1.0", features = ["experimental-simulators"] }
```

They do **not** provide hardware-backed keys, signatures, ML-KEM/Kyber,
confidentiality, quantum resistance, or broker transport. The MQTT encoder
above is independent of the simulated numeric-value formatter and remains only
a packet helper. There are intentionally no aliases named `HsmDevice`,
`PqcKeyPair`, or `MqttDriver`.

## Transports and hardware

Use mature ecosystem crates for anything that touches a network, bus or radio:
[`rumqttc`](https://crates.io/crates/rumqttc) for MQTT on a `std` host or
gateway (it encodes packets itself; send it a serialized `SensorTelemetry`),
[Embassy](https://embassy.dev/) for async embedded firmware,
[`embedded-hal`](https://crates.io/crates/embedded-hal) with your chip's HAL for
GPIO/I2C/SPI, [`tokio-modbus`](https://crates.io/crates/tokio-modbus) for
Modbus RTU/TCP on a host and [`btleplug`](https://crates.io/crates/btleplug) for
BLE from a desktop or gateway. Rullst ships no adapter for these crates and does
not test them with `rullst-iot`; verify each against your target. Embassy
integration has not started.

## Not implemented

- MQTT/CoAP/WebSocket network clients or broker integration.
- ATECC608A, TPM 2.0, or STSAFE hardware backends.
- ML-KEM or any other post-quantum cryptographic primitive.
- Firmware download, delta patching, flash writes, bootloader control, or
  a concrete persistent anti-rollback storage implementation.
- Bidirectional Digital Twin transport or Studio/Nexus device synchronization.

See the [crate roadmap](https://github.com/Rullst/Rullst/blob/v12.1.0/rullst-iot/ROADMAP.md)
for the remaining integration work.

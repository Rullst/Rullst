# Rullst IoT 📡
### *"Embedded Sensor Protocols, Ed25519 OTA Gate & Edge Computing for Rust"*

> [!IMPORTANT]
> The dependency example targets `12.1.0`. Check the [release record](../v12.md)
> for publication status and commit Cargo.lock for reproducible builds.
> Use a path dependency only when intentionally testing checkout-local changes.

`rullst-iot` provides high-assurance telemetry models, bare-metal `#![no_std]`
data structures and packet encoders, and a cryptographically verified
Over-The-Air (OTA) firmware update state machine.

> **Maturity: Experimental.** The API may change between 13.x releases, and
> nothing in this crate has been validated on physical hardware or against a
> live broker. Experimental does not mean broken: the [module status](#module-status)
> below separates working implementations from data models and contracts. See
> the [maturity tiers](../maturity.md).

---

## ⚡ Capability & Lifecycle Matrix

| Subsystem | Lifecycle Status | Description |
| :--- | :---: | :--- |
| **Ed25519 OTA Manifest Gate** | 🟢 `[Implemented / Bounded]` | Verifies a domain-separated signed manifest, target, firmware length/hash, and monotonic counter. A `no_std` store trait adds durable compare-and-set coordination; its concrete persistence, flashing, bootloader handoff, and hardware validation remain external. |
| **`no_std` Telemetry Models** | 🟢 `[Implemented / Bounded]` | Allocation-conscious telemetry, digital-twin, and sensor models are available without `std`; board- and toolchain-specific builds must still be validated in the release matrix. |
| **Protocol Frame Helpers** | 🟢 `[Implemented / Bounded]` | MQTT 5 PUBLISH, RFC 7252 CoAP base requests, Modbus CRC, I2C frame packing, BLE GATT data models, and power-policy abstractions; these are bounded packet/state helpers, not network, bus, or radio drivers. |
| **Experimental Fixtures** | 🟡 `[Simulated fixtures]` | The opt-in feature exposes explicitly named deterministic MQTT formatting, HSM-byte, and PQC-byte fixtures. GPIO/I2C/BLE types are always-available state/frame helpers, not hardware simulators. |
| **Native MQTT/CoAP Transport** | 🔵 `[Roadmap]` | Connections, TLS/DTLS, broker negotiation, acknowledgement/retransmission state, subscriptions, block-wise transfer, and interoperability. |
| **Hardware Security Module (HSM)** | 🔵 `[Roadmap]` | Native secure-element driver interfaces (ATECC608A, TPM 2.0, STSAFE). |

---

## Module status

Each module falls into one of four kinds:

- **Working**: complete for its stated, bounded scope and covered by tests.
- **Contract**: a trait your platform code implements; the crate supplies no
  implementation.
- **Model only**: data structures that hold state in memory. They drive no
  hardware, radio or network and act as placeholders for a platform adapter.
- **Fixture**: deterministic test data behind the `experimental-simulators`
  feature. Never use it in production.

| Module | Kind | What it does | What it does not do |
| :--- | :--- | :--- | :--- |
| `ota` | Working + Contract | `OtaManager` verifies an Ed25519 signature over a domain-separated manifest that binds target, version, rollback counter, firmware length and SHA-256, then names the inactive A/B partition. `RollbackCounterStore` is the contract for durable compare-and-set of the counter. | No download, flash writes, bootloader handoff or concrete counter storage. The deprecated `new`, `verify_signature` and `commit_update` always fail. |
| `mqtt` | Working (encoder) | `MqttPublish` encodes one MQTT 5 PUBLISH packet: QoS 0/1/2, retain, DUP, topic validation and a 1 MiB ceiling. | No CONNECT, SUBSCRIBE, acknowledgement state, socket, TLS or retry. |
| `coap` | Working (encoder) | `CoapRequest` encodes an RFC 7252 GET/POST/PUT/DELETE request with Uri-Path and Content-Format options under a 1152-byte ceiling. | No UDP/DTLS, retransmission, block-wise transfer or response decoding. |
| `modbus` | Working (narrow) | Modbus CRC-16 and a Read Holding Registers (`0x03`) RTU request frame. | No response parsing and no write-request builder (`WriteSingleRegister` is only an enum value). It does not check the protocol's register-count limit and has no serial or TCP transport. |
| `i2c` | Working (narrow) | Builds the byte layout of a register-read transaction, rejecting reserved or non-7-bit addresses and reads above 8,192 bytes. | No bus access. |
| `anomaly` | Working | Classifies a reading against a fixed mean and tolerance (normal, warning beyond 1×, critical beyond 2×); non-finite values fail closed as critical. | No learned baseline or time-series analysis. |
| `twin` | Working (in memory) | `DigitalTwin` stores readings, returns the latest per metric and serializes a JSON snapshot, refusing non-finite values. | No synchronization or transport. It keeps every ingested reading, so trim `readings` on long-running hosts. |
| `ui` | Working | Renders an HTML-escaped sensor snapshot card. | No live updates or connectivity status. |
| `power` | Working (policy) | Recommends a power mode and harvester state from hard-coded millivolt thresholds. | Does not control sleep, wake, charging or solar hardware. |
| `mesh` | Model only | In-memory node list; `best_relay` picks the online node with the strongest recorded RSSI. | No discovery, routing, failure detection or mesh transport. |
| `ble` | Model only | GATT service and characteristic structs. | No radio and no GATT server. |
| `gpio` | Model only | In-memory pin mode and state. | No register or pin access. |
| `hsm` | Fixture | `SimulatedHsmDevice` derives deterministic SHA-256 bytes. | Not an HSM: no key protection, signatures or MACs. |
| `pqc` | Fixture | `SimulatedPqcFixture` derives deterministic hash bytes. | No ML-KEM/Kyber, no encapsulation and no quantum resistance. |

`SensorTelemetry` (crate root) is the shared telemetry data model. The `ota`,
`modbus` and `anomaly` modules also have fuzz targets, and CI builds the crate
for Cortex-M and RISC-V targets. Those builds are compile evidence, not hardware
tests.

### Transports and hardware: use maintained ecosystem crates

For anything that touches a network, bus or radio, use a mature ecosystem crate
and pass Rullst's models or encoded bytes to it. Some starting points:

- **MQTT on a host or gateway with `std`:** [`rumqttc`](https://crates.io/crates/rumqttc).
  A full client encodes packets itself, so give it the topic and a serialized
  `SensorTelemetry` payload. Use `MqttPublish` only when you own the socket and
  need the raw PUBLISH bytes.
- **Async embedded firmware:** [Embassy](https://embassy.dev/) for executors,
  timers and networking (`embassy-net`).
- **GPIO, I2C and SPI drivers:** the [`embedded-hal`](https://crates.io/crates/embedded-hal)
  traits with your chip's HAL. A HAL's `write_read` takes the address and
  register directly, so `I2cHelper` frames are mostly useful for logging, tests
  and adapters that expect a raw byte layout.
- **Modbus RTU/TCP on a host:** [`tokio-modbus`](https://crates.io/crates/tokio-modbus).
- **BLE from a desktop or gateway:** [`btleplug`](https://crates.io/crates/btleplug).

Rullst ships no adapter for these crates and does not test them together with
`rullst-iot`. They are suggestions; check each one against your target and
security requirements. Embassy integration (roadmap milestone M25) has not
started.

---

## 🛡️ Over-The-Air (OTA) Firmware Verification

The `OtaManager` enforces a **fail-closed eligibility gate**: its state machine does not produce an `OtaCommit` receipt before strict Ed25519 verification of the signed manifest. The `RollbackCounterStore` path also requires an exact, strictly increasing compare-and-set. The receipt selects the intended inactive partition; platform code must still flash, verify, implement durable storage, configure the bootloader, and recover safely from power loss.

### The Cryptographic Invariant

```text
[Signed Firmware Manifest]
├── Target Hardware ID: "stm32-sensor-node-v1"
├── Version String:     "2.4.0"
├── Rollback Counter:   12  (Must be strictly > current committed counter)
├── Firmware Length:    131072 bytes
└── Firmware SHA-256:   [32 bytes hash]
                     │
                     ▼
       [Ed25519 Strict Signature Check]
                     │
           ┌─────────┴─────────┐
        Passed               Failed
           │                   │
  [Ready to Commit]     [Revert & Reject]
```

### Usage Example

```rust
use rullst_iot::{
    BootPartition, OtaCommit, OtaError, OtaManager, OtaManifest, RollbackCounterStore,
};

fn process_incoming_ota<S: RollbackCounterStore>(
    firmware_bytes: &[u8],
    signature_bytes: &[u8],
    provisioned_public_key: [u8; 32],
    counter_store: &mut S,
    running_partition: BootPartition,
) -> Result<OtaCommit, OtaError> {
    // 1. Construct the expected manifest from the payload
    let manifest = OtaManifest::from_firmware(
        "esp32-sensor-node", 
        "2.0.0", 
        15, // Proposed monotonic counter
        firmware_bytes
    )?;

    // 2. Load the last committed counter from the platform adapter
    //    and the bank the platform bootloader started
    let mut manager = OtaManager::new_with_running_partition(
        "esp32-sensor-node",
        "1.9.0",
        running_partition,
        provisioned_public_key,
        counter_store,
    )?;

    // 3. Cryptographically verify signature, target, and anti-rollback state
    manager.verify_update(&manifest, firmware_bytes, signature_bytes)?;

    // 4. Flash and read back this bank using platform code before commit.
    let target_partition = manager.verified_target_partition()?;

    // 5. Durable CAS succeeds before local state changes. Coordinate the
    // receipt with the platform bootloader after this call.
    let receipt = manager.commit_verified_update_with_store(counter_store)?;
    debug_assert_eq!(receipt.target_partition(), target_partition);
    Ok(receipt)
}
```

`verified_target_partition` and the receipt always name the bank opposite
`current_partition`. The v13 `new_with_running_partition` constructor takes the
bank the bootloader started; `new_with_counter_store` and
`new_with_trusted_key` assume `PartitionA`, so with them platform code must set
`current_partition` before verifying an update. Committing
does not change `current_partition`; until the platform reboots, a further
update verified in the same process targets the same inactive bank.

The store contract requires power-loss-safe persistence before returning
success. If a store reports a failure after committing, for example because an
acknowledgement was lost, a retry of that same manifest completes the commit
once the store reports, and a fresh `load` confirms, exactly its counter. A
stale manager, or a different image with the same counter, still receives the
conflict. The framework tests
restart/replay, transient retry, corruption and stale-writer conflict at the
adapter boundary, but those tests do not certify a particular flash, secure
element or board. A failure after the durable counter advances can require
platform recovery and a newer signed update.

---

## 🔌 Embedded Bare-Metal Telemetry (`#![no_std]`)

`rullst-iot` exposes a `no_std` model layer intended for constrained microcontrollers. Compatibility is feature-, target-, allocator-, and toolchain-dependent and must be confirmed for the actual board:

```rust
use rullst_iot::SensorTelemetry;

fn sample_reading() -> SensorTelemetry {
    SensorTelemetry::new(
        "node-1",
        "temperature_celsius",
        24.5,
        1_724_500_000,
    )
}
```

### MQTT and CoAP packet boundaries

```rust
use rullst_iot::{
    CoapMessageType, CoapMethod, CoapRequest, MqttPublish, MqttQos,
};

let publish = MqttPublish::reliable(
    "nodes/node-1/temperature",
    b"24.5".to_vec(),
    MqttQos::AtLeastOnce,
    7,
)?
.encode()?;

let request = CoapRequest::new(
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

These helpers follow the local [OASIS MQTT 5 PUBLISH](https://docs.oasis-open.org/mqtt/mqtt/v5.0/mqtt-v5.0.html)
and [RFC 7252 CoAP](https://www.rfc-editor.org/rfc/rfc7252.html) packet shapes.
They deliberately stop before any socket or session state; the transport owns
security, identity, timing, correlation, retry and peer interoperability.

---

## 🔬 Experimental Simulators (`experimental-simulators`)

For local integration tests without physical hardware attached, enable the simulator feature:

```toml
[dependencies]
rullst-iot = { version = "12.1.0", features = ["experimental-simulators"] }
```

This exposes `SimulatedMqttPayloadFormatter`, `SimulatedHsmDevice`, and `SimulatedPqcFixture` for deterministic sandbox execution.

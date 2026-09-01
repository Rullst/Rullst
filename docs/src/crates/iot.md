# Rullst IoT 📡
### *"Embedded Sensor Protocols, Ed25519 OTA Gate & Edge Computing for Rust"*

> [!IMPORTANT]
> The dependency example uses `12.0.0-rc.1`, the planned first v12 RC. Do not
> request it from crates.io before it is published; use a path dependency from
> this source checkout during development.

`rullst-iot` provides high-assurance telemetry models, bare-metal `#![no_std]`
data structures and packet encoders, and a cryptographically verified
Over-The-Air (OTA) firmware update state machine.

---

## ⚡ Capability & Lifecycle Matrix

| Subsystem | Lifecycle Status | Description |
| :--- | :---: | :--- |
| **Ed25519 OTA Manifest Gate** | 🟢 `[Implemented / Bounded]` | Verifies a domain-separated signed manifest, target, firmware length/hash, and monotonic counter. A `no_std` store trait adds durable compare-and-set coordination; its concrete persistence, flashing, bootloader handoff, and hardware validation remain external. |
| **`no_std` Telemetry Models** | 🟢 `[Implemented / Bounded]` | Allocation-conscious telemetry, digital-twin, and sensor models are available without `std`; board- and toolchain-specific builds must still be validated in the release matrix. |
| **Protocol Frame Helpers** | 🟢 `[Implemented / Bounded]` | MQTT 5 PUBLISH, RFC 7252 CoAP base requests, Modbus CRC, I2C frame packing, BLE GATT data models, and power-policy abstractions; these are bounded packet/state helpers, not network, bus, or radio drivers. |
| **Experimental Fixtures** | 🟡 `[Simulador Dev]` | The opt-in feature exposes explicitly named deterministic MQTT formatting, HSM-byte, and PQC-byte fixtures. GPIO/I2C/BLE types are always-available state/frame helpers, not hardware simulators. |
| **Native MQTT/CoAP Transport** | 🔵 `[Roadmap]` | Connections, TLS/DTLS, broker negotiation, acknowledgement/retransmission state, subscriptions, block-wise transfer, and interoperability. |
| **Hardware Security Module (HSM)** | 🔵 `[Roadmap]` | Native secure-element driver interfaces (ATECC608A, TPM 2.0, SE050). |

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
    OtaCommit, OtaError, OtaManager, OtaManifest, RollbackCounterStore,
};

fn process_incoming_ota<S: RollbackCounterStore>(
    firmware_bytes: &[u8],
    signature_bytes: &[u8],
    provisioned_public_key: [u8; 32],
    counter_store: &mut S,
) -> Result<OtaCommit, OtaError> {
    // 1. Construct the expected manifest from the payload
    let manifest = OtaManifest::from_firmware(
        "esp32-sensor-node", 
        "2.0.0", 
        15, // Proposed monotonic counter
        firmware_bytes
    )?;

    // 2. Load the last committed counter from the platform adapter
    let mut manager = OtaManager::new_with_counter_store(
        "esp32-sensor-node",
        "1.9.0",
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

The store contract requires power-loss-safe persistence before returning
success. The framework tests restart/replay, transient retry, corruption and
stale-writer conflict at the adapter boundary, but those tests do not certify a
particular flash, secure element or board. A failure after the durable counter
advances can require platform recovery and a newer signed update.

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
rullst-iot = { version = "12.0.0-rc.1", features = ["experimental-simulators"] }
```

This exposes `SimulatedMqttPayloadFormatter`, `SimulatedHsmDevice`, and `SimulatedPqcFixture` for deterministic sandbox execution.

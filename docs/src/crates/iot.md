# Rullst IoT 📡
### *"Embedded Sensor Protocols, Ed25519 OTA Gate & Edge Computing for Rust"*

`rullst-iot` provides high-assurance telemetry models, bare-metal `#![no_std]` data structures, and a cryptographically verified Over-The-Air (OTA) firmware update state machine.

---

## ⚡ Capability & Lifecycle Matrix

| Subsystem | Lifecycle Status | Description |
| :--- | :---: | :--- |
| **Ed25519 OTA Manifest Gate** | 🟢 `[Implemented / Bounded]` | Verifies a domain-separated signed manifest, target, firmware length/hash, and an in-process anti-rollback counter. Persistent counters, flashing, bootloader handoff, and hardware validation remain external. |
| **`no_std` Telemetry Models** | 🟢 `[Implemented / Bounded]` | Allocation-conscious telemetry, digital-twin, and sensor models are available without `std`; board- and toolchain-specific builds must still be validated in the release matrix. |
| **Protocol Frame Helpers** | 🟢 `[Implemented / Bounded]` | Modbus CRC, I2C frame packing, BLE GATT data models, and power-policy abstractions; these are protocol/state helpers, not physical bus or radio drivers. |
| **Hardware Simulators** | 🟡 `[Simulador Dev]` | Deterministic in-memory GPIO, I2C, and BLE simulators for local testing (`feature = "experimental-simulators"`). |
| **Native MQTT 5.0 Transport** | 🔵 `[Roadmap]` | High-performance asynchronous MQTT 5.0 client integrated via `rumqttc` with QoS 0/1/2. |
| **Hardware Security Module (HSM)** | 🔵 `[Roadmap]` | Native secure-element driver interfaces (ATECC608A, TPM 2.0, SE050). |

---

## 🛡️ Over-The-Air (OTA) Firmware Verification

The `OtaManager` enforces a **fail-closed eligibility gate**: its state machine does not produce an `OtaCommit` receipt before strict Ed25519 verification of the signed manifest. The receipt selects the intended inactive partition; platform code must still flash, verify, persist the counter, configure the bootloader, and recover safely from power loss.

### The Cryptographic Invariant

```
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
use rullst_iot::{OtaManager, OtaManifest};

fn process_incoming_ota(
    firmware_bytes: &[u8],
    signature_bytes: &[u8],
    provisioned_public_key: [u8; 32],
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Construct the expected manifest from the payload
    let manifest = OtaManifest::from_firmware(
        "esp32-sensor-node", 
        "2.0.0", 
        15, // Proposed monotonic counter
        firmware_bytes
    )?;

    // 2. Initialize manager with the last durably committed counter
    let mut manager = OtaManager::new_with_trusted_key(
        "esp32-sensor-node",
        "1.9.0",
        14, // Current hardware counter
        provisioned_public_key,
    )?;

    // 3. Cryptographically verify signature, target, and anti-rollback state
    manager.verify_update(&manifest, firmware_bytes, signature_bytes)?;

    // 4. Select the inactive partition after verification. This does not flash it.
    let commit_receipt = manager.commit_verified_update()?;
    println!("Update verified! Target partition: {:?}", commit_receipt.target_partition());

    Ok(())
}
```

---

## 🔌 Embedded Bare-Metal Telemetry (`#![no_std]`)

`rullst-iot` exposes a `no_std` model layer intended for constrained microcontrollers. Compatibility is feature-, target-, allocator-, and toolchain-dependent and must be confirmed for the actual board:

```rust
#![no_std]
use rullst_iot::telemetry::SensorReading;

let reading = SensorReading {
    sensor_id: 1,
    temperature_celsius: 24.5,
    humidity_percent: 60.2,
    timestamp_epoch: 1724500000,
};
```

---

## 🔬 Experimental Simulators (`experimental-simulators`)

For local integration tests without physical hardware attached, enable the simulator feature:

```toml
[dependencies]
rullst-iot = { version = "12.0.0", features = ["experimental-simulators"] }
```

This exposes `SimulatedMqttPayloadFormatter`, `SimulatedHsmDevice`, and `SimulatedPqcFixture` for deterministic sandbox execution.

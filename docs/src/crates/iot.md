# Rullst IoT 📡
### *"Embedded Sensor Protocols, Ed25519 OTA Gate & Edge Computing for Rust"*

> [!IMPORTANT]
> This page describes the v13 prerelease, published as `13.0.0-alpha.4`. Cargo
> never selects a prerelease on its own, so its dependency examples pin that
> version exactly. The v12 line (`rullst-iot@12`) reached end of life on
> 16 October 2026; see the [release record](../v12.md). Use a path dependency only for checkout-local
> review. Commit `Cargo.lock` for reproducible builds.

`rullst-iot` provides high-assurance telemetry models, bare-metal `#![no_std]`
data structures and packet encoders, and a cryptographically verified
Over-The-Air (OTA) firmware update state machine.

> **Maturity: Experimental.** The API may change between 13.x releases. One
> board has run it: a BBC micro:bit V2 (nRF52833) passed the crate's encoder,
> OTA and counter-store checks on 2026-11-10 ([hardware
> validation](#hardware-validation-bbc-microbit-v2)). Nothing has been tested
> against a live broker, on another board or with external flash.
> Experimental does not mean broken: the [module status](#module-status)
> below separates working implementations from data models and contracts. See
> the [maturity tiers](../maturity.md).

---

## ⚡ Capability & Lifecycle Matrix

| Subsystem | Lifecycle Status | Description |
| :--- | :---: | :--- |
| **Ed25519 OTA Manifest Gate** | 🟢 `[Implemented / Bounded]` | Verifies a domain-separated signed manifest, target, firmware length/hash, and monotonic counter. A `no_std` store trait adds durable compare-and-set coordination; its concrete persistence, flashing, bootloader handoff, and hardware validation remain external. |
| **`no_std` Telemetry Models** | 🟢 `[Implemented / Bounded]` | Allocation-conscious telemetry, digital-twin, and sensor models are available without `std` (with `alloc` and a global allocator). CI links a firmware for `thumbv7em-none-eabihf`, and the same firmware ran on a micro:bit V2; other boards and toolchains must still be validated. |
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

> **Changed in 13.0.0-alpha.5.** `OtaManager`'s state is read with the
> methods `current_partition()`, `status()` and `firmware_version()`; the
> fields of those names are private, so only verification and commit change
> them. Code that assigned `current_partition` calls
> `with_running_partition(bank)` instead. `rullst-iot` is
> [Experimental](../maturity.md), which allows this break; the migration
> guide lists it.

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
`current_partition()`. The v13 `new_with_running_partition` constructor takes
the bank the bootloader started; `new_with_counter_store` and
`new_with_trusted_key` assume `PartitionA`, so with them platform code calls
`with_running_partition(bank)` (13.0.0-alpha.5) before verifying an update,
which also discards an update verified for the other bank. Committing does
not change `current_partition()`; until the platform reboots, a further
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

`rullst-iot` exposes a `no_std` model layer intended for constrained microcontrollers. Compatibility is feature-, target-, allocator-, and toolchain-dependent and must be confirmed for the actual board.

Depend on it with `default-features = false`. The crate uses `alloc`, so the
firmware must provide a `#[global_allocator]` (for example
[`embedded-alloc`](https://crates.io/crates/embedded-alloc)) as well as its own
panic handler. The heap it needs is small: the encoders allocate their output
(a packet is a second copy of its payload) and OTA verification allocates the
manifest's signing bytes; the micro:bit run peaked at 1,153 bytes. When the
allocator refuses a buffer, `MqttPublish::encode`, `CoapRequest::encode` and
`path_segment` return `AllocationFailed`, and `OtaManifest::signing_bytes` and
`OtaManager::verify_update` return `OtaError::AllocationFailed`, instead of
aborting. Reserve about 8 KiB of stack for `verify_update` (the Ed25519
verification) and about 2.6 KiB for building an `OtaManager` (decompressing
the public key), plus your own frames:

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

## Hardware validation (BBC micro:bit V2)

On 2026-11-10 a BBC micro:bit V2 (nRF52833, Cortex-M4F at 64 MHz, 128 KiB
RAM, 512 KiB flash) ran a firmware built on `rullst-iot` with
`default-features = false`, `embedded-alloc` and a 32 KiB heap, on Rust 1.99.1
at `opt-level = 3` with LTO. Its checks ran twice per boot, on every boot,
with no failure:

| Group | Checks | What ran on the device |
| :--- | :---: | :--- |
| Telemetry and frames | 12 | `SensorTelemetry` and `DigitalTwin` JSON written and read back, Modbus RTU frame and CRC, I2C read frame; non-finite readings and malformed JSON rejected |
| MQTT 5 PUBLISH | 6 | QoS 0/1/2, RETAIN, DUP and a two-byte Remaining Length; wildcard topics, packet identifier 0 and DUP on QoS 0 rejected |
| CoAP | 8 | CON/NON GET, POST, PUT and DELETE with extended option lengths and a two-byte Content-Format; oversized tokens, segments and payloads, and `..`, rejected |
| Anomaly detector | 2 | A 12-sample series, including NaN and infinity, and invalid configurations |
| Signed OTA | 21 | Valid update verified and committed through a store; tampered image, signature or version, truncated image, short signature, wrong key, wrong target, lower or equal rollback counter and a weak key rejected; partition selection from bank A and B |
| Rollback-counter store | 9 | Compare-and-set success, stale expected value, non-increasing value, two managers committing from one store, replay after restart, a lost acknowledgement completed on retry, an unavailable store |
| Allocation failure | 2 | A packet larger than the free heap returns `AllocationFailed`, and the next one encodes |

The MQTT, CoAP, JSON, Modbus, I2C, SHA-256 and signing-byte output was
byte-identical to the `std` build on the host. Measured with the DWT cycle
counter (cross-checked against a 1 MHz timer):

| Operation | Time |
| :--- | ---: |
| SHA-256 of a 32 KiB image (`OtaManifest::from_firmware`) | 26.0 ms |
| `verify_update`, 32 KiB image | 62.7 ms |
| `verify_update`, 16-byte image (mostly Ed25519) | 36.9 ms |
| `MqttPublish` / `CoapRequest` encode, about 40 bytes | under 0.1 ms |
| Rejection before hashing (wrong target, truncated image, malformed signature) | under 0.01 ms |

At `opt-level = "z"` the two verifications took 83.3 ms and 53.3 ms. The
whole test firmware, including the test harness, a 32 KiB test image and the
expected vectors, used about 255 KB of flash (162 KB at `opt-level = "z"`). The heap
peaked at 1,153 bytes, OTA verification itself needing 143. Peak stack depth:

| Call | Stack |
| :--- | ---: |
| `OtaManager::verify_update` | 7,656 B |
| `OtaManager::new_with_running_partition` | 2,600 B |
| `serde_json::from_slice::<SensorTelemetry>` | 492 B |
| `OtaManifest::from_firmware` | 400 B |
| `commit_verified_update_with_store` | 360 B |
| MQTT / CoAP encode | 180 B / 196 B |

**Faster verification.** `rullst-iot` uses `ed25519-dalek` without its
precomputed tables. An application can enable them through Cargo feature
unification by adding
`ed25519-dalek = { version = "2.2", default-features = false, features = ["fast"] }`
to its own dependencies. On the micro:bit this made verification about 7%
faster (34.2 ms and 59.8 ms), its stack 1.6 KiB smaller (6,144 B) and the
firmware 7.5 KiB larger.

**Hashed twice.** A device builds the manifest with
`OtaManifest::from_firmware`, which hashes the image, and `verify_update`
hashes it again, so verification costs two passes (about 26 ms per 32 KiB
each), and the image must be one contiguous `&[u8]`: memory-mapped internal
flash works, external SPI flash or a streamed download does not. A manifest
built from its received fields and streaming verification are planned for
13.1 (see the [crate roadmap](https://github.com/Rullst/Rullst/blob/main/rullst-iot/ROADMAP.md)).

**Not validated:** MQTT or CoAP transports, TLS/DTLS or a broker; other
boards and architectures (ESP32, RISC-V, Cortex-M0); external flash and
streamed images; a persistent rollback-counter store (the run used one in
RAM), flashing, bootloader handoff and power loss.

The firmware is in the repository at
[`.github/fixtures/iot-microbit-v2`](https://github.com/Rullst/Rullst/tree/main/.github/fixtures/iot-microbit-v2).
CI builds it for `thumbv7em-none-eabihf` and reports its size, so a `no_std`
regression fails the build; its `src/main.rs` says how to flash it and read
the results from the board's USB serial port.

---

## 🔬 Experimental Simulators (`experimental-simulators`)

For local integration tests without physical hardware attached, enable the simulator feature:

```toml
[dependencies]
rullst-iot = { version = "=13.0.0-alpha.4", features = ["experimental-simulators"] }
```

This exposes `SimulatedMqttPayloadFormatter`, `SimulatedHsmDevice`, and `SimulatedPqcFixture` for deterministic sandbox execution.

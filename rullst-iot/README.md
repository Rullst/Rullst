# rullst-iot

`rullst-iot` provides `no_std`-compatible telemetry models, protocol frame
builders, deterministic edge helpers, and a fail-closed signed firmware gate.

**Maturity: Experimental** ([tiers](https://rullst.github.io/Rullst/book/maturity.html)).
The API may change between 13.x releases. On 2026-11-10 a BBC micro:bit V2
(nRF52833) ran the encoders, the signed OTA gate and the counter-store
contract with no failure ([hardware validation](#hardware-validation)), and
the `std` telemetry, MQTT encoder and anomaly path ran end to end with an
Arduino Uno sensor and a Rullst gateway and dashboard, with no broker
involved. No other board, live broker or external flash has been tested. Experimental does
not mean broken; the table below shows which modules are working
implementations.

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
| `anomaly` | Working | Fixed mean/tolerance classification; non-finite values fail closed. `AnomalyState` displays and serializes (`"critical_anomaly"`). |
| `twin` | Working (in memory) | Readings, latest per metric, JSON snapshot. No sync; readings grow until you trim them. |
| `ui` | Working | Escaped HTML snapshot card (`IotHtml`), reading shown with at most two decimals. |
| `power` | Working (policy) | Hard-coded voltage thresholds to a recommended mode. Controls no hardware. |
| `mesh` | Model only | Node list and strongest-RSSI relay pick. No routing or transport. |
| `ble` | Model only | GATT data structs. No radio. |
| `gpio` | Model only | In-memory pin state. No register access. |
| `hsm`, `pqc` | Fixture | Deterministic hash bytes. Not an HSM and not post-quantum cryptography. |

## Implemented scope

- `SensorTelemetry` and `DigitalTwin` in-memory state models.
  `SensorTelemetry::timestamp` is whole UNIX seconds; a reading that needs
  finer time carries it in the application's own payload type.
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
  snapshot rather than inferring device connectivity. It returns `IotHtml`, the
  crate's own escaped-HTML type (this crate does not depend on the web
  runtime): insert it into a Rullst view with
  `Markup::from_trusted(card.into_string())`. The reading is shown with at most
  two decimals, in scientific notation outside 0.01 to 10^9, and as `n/a` when
  it is not finite.
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

## Signing firmware with the CLI

`cargo rullst iot:keygen --out keys/ota.key` writes an Ed25519 key pair (the
secret with mode `0600`; the public key as a Rust `const`, raw bytes and
hexadecimal), and `cargo rullst iot:sign --key keys/ota.key --image fw.bin
--target board-revision-a --version 1.1.0 --counter 121` writes `fw.bin.manifest`
(the bytes `OtaManifest::from_signing_bytes` parses) and `fw.bin.manifest.sig`
(the signature for `begin_update`). Both use this crate's encoding, and
`iot:sign` checks its output with `OtaManager::begin_update` before writing it.

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
a 1 MiB local ceiling. Both encoders allocate their output fallibly and return
`AllocationFailed` (`MqttCodecError`, `CoapCodecError`, both
`#[non_exhaustive]` since 13.0.0-alpha.5) when the allocator refuses it, as on
a device whose heap is smaller than the packet; an encoded packet is a second
copy of its payload. It does not implement CONNECT, broker negotiation,
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

/// The manifest bytes and the signature arrive first; the image follows in
/// chunks, from the network or read back from the inactive bank.
fn install<'c, S: RollbackCounterStore>(
    manifest_bytes: &[u8],
    signature: &[u8],
    image_chunks: impl IntoIterator<Item = &'c [u8]>,
    trusted_public_key: [u8; 32],
    counter_store: &mut S,
    running_partition: BootPartition,
) -> Result<OtaCommit, OtaError> {
    // The exact bytes the publisher signed (`OtaManifest::signing_bytes`).
    let manifest = OtaManifest::from_signing_bytes(manifest_bytes)?;
    // `running_partition` is the bank the platform bootloader started.
    let mut ota = OtaManager::new_with_running_partition(
        "board-revision-a",
        "1.0.0",
        running_partition,
        trusted_public_key,
        counter_store,
    )?;

    // Target, rollback counter and signature are checked before any image byte.
    let mut verifier = ota.begin_update(&manifest, signature)?;
    for chunk in image_chunks {
        verifier.update(chunk)?;
    }
    verifier.finish()?; // the length and the SHA-256 digest
    let target = ota.verified_target_partition()?;

    // The platform coordinates the returned receipt with its bootloader.
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
`current_partition()`. The v13 constructor `new_with_running_partition` takes
the bank the bootloader started. `new_with_counter_store` and
`new_with_trusted_key` assume `PartitionA`, so with them the platform calls
`with_running_partition(bank)` with the bank its bootloader reports before
verifying an update once the device has rebooted into `PartitionB`; that
also discards an update verified for the other bank.
Committing does not change `current_partition()`: the device keeps running the
old bank until the platform reboots, so a further update verified in the same
process targets the same inactive bank, never the running one.
Since 13.0.0-alpha.5 the manager's state is read with `current_partition()`,
`status()` and `firmware_version()`; the fields are private.

A device builds the manifest from what it receives, without the image:
`OtaManifest::from_signing_bytes` parses the canonical bytes the publisher
signed (`signing_bytes()`), and `OtaManifest::new` takes the fields one by one.
`begin_update` checks the target, the rollback counter and the Ed25519
signature before any image byte, then returns an `OtaImageVerifier`, which
hashes the image chunk by chunk, refuses a chunk past the signed length and,
in `finish`, checks the length and the SHA-256 digest. The image is hashed
once and never has to be one slice in memory, so it can stream from the
network or from external flash. Feed it the bytes as they are in the inactive
bank (read back after writing), so the digest covers what will boot. While the
verifier exists the manager is `Downloading`; dropping it unfinished leaves the
manager `Failed`. `verify_update(&manifest, firmware, signature)` does the
same for an image held as one slice (for example memory-mapped internal
flash). `OtaManifest::from_firmware` hashes the image; it is for the
publisher.

`OtaManager::new`, `verify_signature`, and `commit_update` are deprecated
migration APIs. All three always return `OtaError` because keyless construction,
payload-only verification, and unconditional commit cannot provide the required
guarantees.

## `no_std` use

Depend on the crate with `default-features = false`. It uses `alloc`, so the
firmware provides a `#[global_allocator]` (for example `embedded-alloc`) and a
panic handler. Plan for about 8 KiB of stack in `begin_update` or
`verify_update` and 2.6 KiB while building an `OtaManager`, plus your own
frames; the heap needs little
beyond the packets you encode. `rullst-iot` uses `ed25519-dalek` without its
precomputed tables; adding `ed25519-dalek = { version = "2.2",
default-features = false, features = ["fast"] }` to the application turns them
on (about 7% faster verification and 1.6 KiB less stack on a Cortex-M4, for
7.5 KiB more flash).

## Hardware validation

A BBC micro:bit V2 (nRF52833, Cortex-M4F at 64 MHz, 128 KiB RAM) ran 66
checks, twice per boot, on 2026-11-10: telemetry JSON, digital-twin, Modbus
and I2C frames, MQTT 5 and CoAP encodings byte-identical to the host build,
the anomaly detector, signed OTA verification (valid, tampered, wrong key,
wrong target, rollback), streaming verification of a 32 KiB image in 4 KiB
chunks from a manifest built from received fields, the rollback-counter
compare-and-set contract with an in-RAM store, and allocation failure. The
streamed verification took 62.8 ms (36.6 ms for the signature before the first
chunk, 3.3 ms per chunk) with 7.6 KiB of stack; the heap peaked at 1,153 bytes.
Transports, other boards, external flash, a persistent counter store and the
bootloader remain unvalidated. On the same day the `std` build ran end to end on a host: readings from an
Arduino Uno sensor went through `SensorTelemetry`, the MQTT 5 PUBLISH encoder
and `AnomalyDetector` to a Rullst gateway and dashboard. No broker was
involved, so the encoded packets were not delivered to one. The
[crate guide](https://rullst.github.io/Rullst/book/crates/iot.html#hardware-validation-bbc-microbit-v2)
has the full results; the firmware is
[`.github/fixtures/iot-microbit-v2`](https://github.com/Rullst/Rullst/tree/main/.github/fixtures/iot-microbit-v2),
which CI builds for `thumbv7em-none-eabihf`.

## Experimental simulators

The `experimental-simulators` feature exposes deterministic fixtures named
`SimulatedHsmDevice`, `SimulatedPqcFixture`, and
`SimulatedMqttPayloadFormatter`. They are useful only in tests and demos:

```toml
# The v13 prerelease, pinned exactly; the v12 line reached end of life.
rullst-iot = { version = "=13.0.0-alpha.5", features = ["experimental-simulators"] }
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

See the [crate roadmap](https://github.com/Rullst/Rullst/blob/main/rullst-iot/ROADMAP.md)
for the remaining integration work.

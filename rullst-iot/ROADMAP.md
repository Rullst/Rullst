# rullst-iot roadmap

> **Status policy (2026-08-26):** this roadmap already uses scoped checkboxes;
> the audited [`rullst-iot` row](../ROADMAP.md#audit-of-the-detailed-crate-roadmaps)
> and [capability ledger](../docs/src/capability-ledger.md) are the canonical
> cross-framework interpretation.

This roadmap distinguishes data/frame helpers from network, hardware, and
cryptographic implementations.

## Available

- [x] `no_std` telemetry models and deterministic state helpers.
- [x] Modbus CRC/frame construction, I2C frame construction, BLE GATT data
  structures, anomaly evaluation, power policy, and topology models.
- [x] Strict Ed25519 verification over a domain-separated firmware manifest.
- [x] Manifest binding for target, version, firmware length, SHA-256 digest, and
  monotonic anti-rollback counter.
- [x] Fail-closed OTA state transition: commit is rejected until verification.
- [x] `no_std` rollback-counter adapter contract with exact monotonic
  compare-and-set, typed failure, restart/replay, retry, and stale-writer tests.
- [x] Negative tests and fuzz coverage for malformed/untrusted OTA input.
- [x] Safe `make:iot` telemetry scaffold with feature/module registration,
  collision refusal, identifier validation, and a materialized compile test.
- [x] Escaped local HTML snapshot cards and fail-closed anomaly handling for
  non-finite sensor/configuration values.

## Required for an end-to-end OTA implementation

- [ ] Authenticated manifest transport and a stable wire format.
- [ ] Streaming firmware hashing with bounded memory.
- [ ] Platform flash writer, read-back validation, and power-loss-safe A/B state.
- [ ] A concrete hardware-backed durable counter implementation and atomic
  bootloader selection, with power-loss/fault-injection device tests.
- [ ] Key rotation/revocation policy and recovery-key provisioning.

## Protocol and hardware integrations

- [ ] MQTT 5 client and broker interoperability tests.
- [ ] CoAP/LwM2M and LoRaWAN transports.
- [ ] Real GPIO, I2C, BLE, Modbus, and mesh hardware adapters.
- [ ] Audited ATECC608A, TPM 2.0, and STSAFE backends.
- [ ] Audited ML-KEM implementation with published test vectors.
- [ ] OPC-UA, Sparkplug B, SocketCAN/CAN FD, J1939, and OBD-II support.
- [ ] Independent IEC 61508/IEC 62443 assessment; no certification is currently
  claimed.

The opt-in `experimental-simulators` feature is not progress toward compliance:
its `Simulated*` types generate deterministic fixture bytes only.

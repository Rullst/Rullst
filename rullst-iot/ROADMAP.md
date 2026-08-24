# rullst-iot roadmap

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
- [x] Negative tests and fuzz coverage for malformed/untrusted OTA input.

## Required for an end-to-end OTA implementation

- [ ] Authenticated manifest transport and a stable wire format.
- [ ] Streaming firmware hashing with bounded memory.
- [ ] Platform flash writer, read-back validation, and power-loss-safe A/B state.
- [ ] Durable monotonic counter storage and atomic bootloader selection.
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

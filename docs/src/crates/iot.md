# rullst-iot

> **Vision preserved:** MQTT/CoAP/Sparkplug, real flash/boot lifecycle, HSM/PQC,
> Embassy, hardware integration, and HIL testing remain itemized with status and
> recommendation in the [capability ledger](../capability-ledger.md#iot-edge-and-cryptography-vision).

`rullst-iot` contains `no_std`-compatible telemetry models, protocol frame
helpers, edge state models, and a signed firmware verification gate.

## Current capabilities

- Telemetry and digital-twin in-memory models.
- Modbus CRC/frame, I2C frame, BLE GATT data, anomaly, topology, and power-policy
  helpers. These types do not operate hardware or network transports by
  themselves.
- Strict Ed25519 verification of a domain-separated OTA manifest that binds the
  target, version, monotonic rollback counter, firmware size, and SHA-256 hash.
- A fail-closed state machine that refuses partition selection until the exact
  firmware and manifest signature have been verified.

```rust
use rullst_iot::{OtaManager, OtaManifest};

# fn verify_update(
#     firmware: &[u8],
#     signature: &[u8],
#     trusted_public_key: [u8; 32],
# ) -> Result<(), rullst_iot::OtaError> {
let manifest = OtaManifest::from_firmware("board-a", "2.0.0", 8, firmware)?;
let mut manager =
    OtaManager::new_with_trusted_key("board-a", "1.0.0", 7, trusted_public_key)?;

manager.verify_update(&manifest, firmware, signature)?;
let selection = manager.commit_verified_update()?;

// The application must flash/read-back the bank, persist the counter, and
// configure the bootloader. The receipt does not claim those operations ran.
let _target_bank = selection.target_partition();
# Ok(())
# }
```

The trusted public key must be provisioned independently of the update channel.
The monotonic counter supplied to `new_with_trusted_key` must come from durable,
rollback-resistant platform storage.

## Experimental fixtures

Enabling `experimental-simulators` exposes only explicitly named deterministic
fixtures:

- `SimulatedMqttPayloadFormatter` is not an MQTT encoder/client.
- `SimulatedHsmDevice` does not use hardware or create signatures.
- `SimulatedPqcFixture` does not implement ML-KEM/Kyber or confidentiality.

These types have no production aliases. MQTT transport, real HSM backends,
post-quantum cryptography, firmware flashing, bootloader control, and persistent
anti-rollback storage remain roadmap work.

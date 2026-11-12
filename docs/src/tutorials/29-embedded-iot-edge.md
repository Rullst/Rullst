# Tutorial 29: IoT data and frame helpers (`rullst-iot`)

`rullst-iot` provides telemetry/state models and protocol frame builders that
compile without `std`. Some APIs use `alloc`, so a bare-metal application must
supply an allocator.

It also provides bounded MQTT 5 PUBLISH (`MqttPublish`) and CoAP request
(`CoapRequest`) encoders. The crate does not currently read hardware registers
or provide MQTT/CoAP network transport, OPC-UA, Sparkplug B, HSM, or
post-quantum implementations.

> **Experimental.** `rullst-iot` and the `make:iot` generator are in the
> Experimental [maturity tier](../maturity.md): the API may change between 13.x
> releases. One board, a BBC micro:bit V2, has run the encoders and the OTA gate
> ([hardware validation](../crates/iot.md#hardware-validation-bbc-microbit-v2)),
> and the `std` telemetry, MQTT encoder and anomaly path below ran end to end
> with an Arduino Uno sensor and a Rullst gateway and dashboard on 2026-11-10,
> with no broker ([host run](../crates/iot.md#host-end-to-end-run-arduino-uno-and-a-rullst-gateway));
> no other board, broker or network has. The
> [module status](../crates/iot.md#module-status) lists which modules are working
> implementations and which are only data models or contracts.

## Add the crate

In a Rullst application, `cargo rullst make:iot TemperatureSensor` turns on the
`iot` feature of `rullst`, writes `src/iot/temperature_sensor.rs` (a
`TemperatureSensorDevice` that returns `SensorTelemetry` readings) and declares
the `iot` modules. The crate is then reachable as `rullst::iot`, so the
`rullst_iot::` paths below become `rullst::iot::`.

A firmware crate depends on `rullst-iot` directly and turns off its default
`std` feature:

```toml
[dependencies]
rullst-iot = { version = "=13.0.0-alpha.5", default-features = false }
```

## Build a telemetry model and Modbus request

```rust
use rullst_iot::{AnomalyDetector, ModbusFrame, SensorTelemetry};

let telemetry = SensorTelemetry::new(
    "sensor-01",
    "temperature",
    38.5,
    1_700_000_000,
);
let detector = AnomalyDetector::new(25.0, 5.0);
let state = detector.evaluate(telemetry.value);
// `AnomalyState` displays as `critical anomaly` and serializes as
// "critical_anomaly"; the timestamp is whole UNIX seconds.
assert_eq!(state.to_string(), "critical anomaly");

// This builds bytes for a request. Platform code must send them over a real
// serial/TCP transport and handle timeouts, retries, and the response.
let request = ModbusFrame::read_holding_registers(1, 0, 10);
assert_eq!(request.len(), 8);
let _ = state;
```

## Signed firmware artifacts

Sign the image on the publisher's machine with the CLI, then verify it on the
device before selecting an inactive partition:

```console
cargo rullst iot:keygen --out keys/ota.key
cargo rullst iot:sign --key keys/ota.key --image firmware.bin \
  --target sensor-01 --version 1.1.0 --counter 2
```

The device embeds `keys/ota.key.pub.rs` (the public key as a Rust `const`),
parses `firmware.bin.manifest` with `OtaManifest::from_signing_bytes` and
passes it with `firmware.bin.manifest.sig` to `OtaManager::begin_update`,
then streams the image through the returned verifier. See the
[crate guide](../crates/iot.md#signing-firmware-with-the-cli) for the device
code and the trust, persistence, and bootloader requirements that remain the
integrator's responsibility.

## Experimental fixtures

The opt-in `experimental-simulators` feature contains explicitly named
`Simulated*` fixtures. They are deterministic test data generators, not hardware
or protocol implementations.

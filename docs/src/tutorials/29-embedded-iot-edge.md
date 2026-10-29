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
> releases and nothing has been validated on physical hardware. The
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
rullst-iot = { version = "=13.0.0-alpha.4", default-features = false }
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

// This builds bytes for a request. Platform code must send them over a real
// serial/TCP transport and handle timeouts, retries, and the response.
let request = ModbusFrame::read_holding_registers(1, 0, 10);
assert_eq!(request.len(), 8);
let _ = state;
```

## Signed firmware artifacts

Use `OtaManifest` and `OtaManager::new_with_trusted_key` to verify a firmware
artifact before selecting an inactive partition. See the
[crate guide](../crates/iot.md) for the trust, persistence, and bootloader
requirements that remain the integrator's responsibility.

## Experimental fixtures

The opt-in `experimental-simulators` feature contains explicitly named
`Simulated*` fixtures. They are deterministic test data generators, not hardware
or protocol implementations.

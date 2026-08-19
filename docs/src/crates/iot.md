# rullst-iot 🔌

`rullst-iot` is the Embedded IoT & Edge Hardware Supremacy crate of the Rullst Framework. A comprehensive **6-Phase** library providing a `#![no_std]` optional runtime (< 2MB RAM footprint) for Raspberry Pi, ESP32, STM32, and Arduino 32-bit.

---

## 💻 Hardware Compatibility & Targets

| Target Hardware | Architecture | Mode | Supported Protocols | Memory Footprint |
| :--- | :--- | :--- | :--- | :--- |
| **Raspberry Pi / Orange Pi** | ARM64 / ARMv7 | `std` | MQTT, CoAP, WebSockets, Modbus, BLE | < 2MB RAM |
| **ESP32 (ESP32-S3/C3)** | Xtensa / RISC-V | `no_std` | MQTT, CoAP, BLE Telemetry, OTA | < 256KB RAM |
| **STM32 (Cortex-M)** | ARM Cortex-M | `no_std` | Modbus RTU/TCP, CoAP, HSM | < 128KB RAM |
| **Arduino 32-Bit (Due/Nano)** | ARM Cortex-M | `no_std` | BLE, Serial Modbus | < 64KB RAM |

---

## 📦 Features & Modules

### Phase 1 — Core Telemetry & Protocols
- **`SensorTelemetry`**: Unified telemetry payload (temperature, vibration, pressure, etc.).
- **`MqttDriver`**: MQTT / CoAP topic payload formatter.

### Phase 2 — Hardware HAL & Industrial Protocols
- **`GpioPin`**: Cross-platform GPIO digital pin control (`set_high()`, `set_low()`, `read()`).
- **`I2cHelper`**: I2C sensor register transaction builder.
- **`ModbusFrame`**: Modbus RTU/TCP frame builder with **CRC-16**.
- **`GattService`** / **`GattCharacteristic`**: BLE GATT service generator.

### Phase 3 — On-Device Edge AI & Micro-UI
- **`AnomalyDetector`**: Statistical anomaly engine (`Normal` / `Warning` / `CriticalAnomaly`) for `no_std` targets.
- **`IotDashboard`**: Ultra-lightweight HTMX HTML widget generator (< 50KB).

### Phase 4 — Autonomous Swarm & OTA Updates
- **`MeshTopology`** / **`MeshNode`**: Self-healing P2P mesh network with RSSI-based relay path resolution.
- **`OtaManager`**: Zero-trust OTA firmware updates with dual A/B bootloader partition rollback.

### Phase 5 — Hardware Security & Post-Quantum
- **`HsmDevice`**: Hardware Security Element bindings (ATECC608A, TPM 2.0, STSAFE).
- **`PqcKeyPair`**: Post-Quantum ML-KEM / Kyber key encapsulation for quantum-safe telemetry links.

### Phase 6 — Ultra-Low-Power & Digital Twin
- **`PowerGovernor`**: Deep Sleep power governor with solar harvester voltage monitoring.
- **`DigitalTwin`**: Real-time bi-directional sync engine serializing physical/virtual device state as JSON.

### Phase 7 — Industrial Standards & Certification *(Roadmap)*
- **`rullst_iot::opcua`**: Industry 4.0 OPC-UA driver for SCADA/MES/ERP communication (ISA-95, IEC 62541).
- **`rullst_iot::sparkplug`**: MQTT Sparkplug B profile for Unified Namespace / IIoT interoperability.
- **`rullst_iot::safety`**: IEC 61508 / 62443 Safety Mode — deterministic execution with watchdog timer for SIL 2/3 certification.

---

## 🔄 CI/CD Workflows

| Workflow | Trigger | Purpose |
| :--- | :--- | :--- |
| [`no_std-build.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/no_std-build.yml) | Push to `rullst-iot/**` | Validates bare-metal compilation on Cortex-M & RISC-V targets |
| [`iot-integration.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/iot-integration.yml) | Push to `rullst-iot/**` | Runs 18 unit tests + QEMU Cortex-M simulation |
| [`pqc-compliance.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/pqc-compliance.yml) | Push to crypto files + weekly cron | NIST ML-KEM / Kyber & HSM audit, `cargo audit`, unsafe detection |

---

## 🚀 Quickstart

```rust
use rullst_iot::{SensorTelemetry, MqttDriver, AnomalyDetector, DigitalTwin};

fn main() {
    let telemetry = SensorTelemetry::new("esp32-farm-01", "temperature", 38.9, 1700000000);
    println!("MQTT: {}", MqttDriver::format_mqtt_payload(&telemetry));

    let detector = AnomalyDetector::new(25.0, 5.0);
    println!("State: {:?}", detector.evaluate(38.9)); // => CriticalAnomaly

    let mut twin = DigitalTwin::new("esp32-farm-01");
    twin.ingest(telemetry);
    println!("Twin Sync: {}", twin.to_sync_payload());
}
```

## 🛠️ CLI

```bash
cargo rullst make:iot SensorGateway
```

## 🗺️ Roadmap

All 6 phases complete. See [`ROADMAP.md`](https://github.com/Rullst/Rullst/blob/main/rullst-iot/ROADMAP.md).

## 📄 License

MIT OR Apache-2.0.

# rullst-iot 🔌

`rullst-iot` is the Embedded IoT & Edge Hardware Supremacy crate of the Rullst Framework. A comprehensive **6-Phase** library providing `#![no_std]` optional runtime (< 2MB RAM footprint) for single-board computers (Raspberry Pi, Orange Pi) and microcontrollers (ESP32, STM32, Arduino 32-bit).

---

## 💻 Hardware Compatibility & Targets

| Target Hardware | Architecture | Mode | Supported Protocols | Memory Footprint |
| :--- | :--- | :--- | :--- | :--- |
| **Raspberry Pi / Orange Pi** | ARM64 / ARMv7 | `std` | MQTT, CoAP, WebSockets, Modbus, BLE | < 2MB RAM |
| **ESP32 (ESP32-S3/C3)** | Xtensa / RISC-V | `no_std` | MQTT, CoAP, BLE Telemetry, OTA | < 256KB RAM |
| **STM32 (Cortex-M)** | ARM Cortex-M | `no_std` | Modbus RTU/TCP, CoAP, HSM | < 128KB RAM |
| **Arduino 32-Bit (Due/Nano)** | ARM Cortex-M | `no_std` | BLE, Serial Modbus | < 64KB RAM |

---

## 📦 All Modules at a Glance

### Phase 1 — Core Telemetry & Protocols
| Module | Type | Description |
| :--- | :--- | :--- |
| `SensorTelemetry` | Struct | Unified telemetry payload model (temperature, vibration, pressure, etc.) |
| `MqttDriver` | Struct | MQTT / CoAP topic payload formatter |

### Phase 2 — Hardware HAL & Industrial Protocols
| Module | Type | Description |
| :--- | :--- | :--- |
| `GpioPin` | Struct | Cross-platform GPIO digital pin control |
| `I2cHelper` | Struct | I2C sensor register read/write transaction builder |
| `ModbusFrame` | Struct | Modbus RTU/TCP frame builder with automatic **CRC-16** |
| `GattService` | Struct | Bluetooth Low Energy (BLE) GATT service & characteristic generator |

### Phase 3 — On-Device Edge AI & Micro-UI
| Module | Type | Description |
| :--- | :--- | :--- |
| `AnomalyDetector` | Struct | Embedded statistical anomaly engine (`Normal` / `Warning` / `CriticalAnomaly`) |
| `IotDashboard` | Struct | Ultra-lightweight HTMX HTML widget generator (< 50KB) |

### Phase 4 — Autonomous Swarm & OTA Updates
| Module | Type | Description |
| :--- | :--- | :--- |
| `MeshTopology` | Struct | Self-healing P2P mesh topology manager (RSSI-based relay resolution) |
| `OtaManager` | Struct | Zero-Trust OTA firmware updates with dual A/B partition rollback |

### Phase 5 — Hardware Security & Post-Quantum
| Module | Type | Description |
| :--- | :--- | :--- |
| `HsmDevice` | Struct | Hardware Security Element binding (ATECC608A, TPM 2.0, STSAFE) |
| `PqcKeyPair` | Struct | Post-Quantum ML-KEM / Kyber key encapsulation for edge telemetry |

### Phase 6 — Ultra-Low-Power & Digital Twin
| Module | Type | Description |
| :--- | :--- | :--- |
| `PowerGovernor` | Struct | Deep Sleep power governor with solar harvester voltage monitoring |
| `DigitalTwin` | Struct | Real-time bi-directional sync engine for physical/virtual device state |

---

## 🚀 Usage Example

```rust
use rullst_iot::{SensorTelemetry, MqttDriver, AnomalyDetector, DigitalTwin};

fn main() {
    let telemetry = SensorTelemetry::new("esp32-farm-01", "temperature", 38.9, 1700000000);
    let payload = MqttDriver::format_mqtt_payload(&telemetry);
    println!("MQTT Payload: {}", payload);

    // Anomaly detection
    let detector = AnomalyDetector::new(25.0, 5.0);
    println!("State: {:?}", detector.evaluate(38.9)); // => CriticalAnomaly

    // Digital Twin sync
    let mut twin = DigitalTwin::new("esp32-farm-01");
    twin.ingest(telemetry);
    println!("Cloud Sync Payload: {}", twin.to_sync_payload());
}
```

## 🛠️ CLI Scaffolding

```bash
cargo rullst make:iot SensorGateway
# → Creates src/iot/sensorgateway.rs pre-wired with telemetry models
```

---

## 🗺️ Roadmap

All 6 phases complete. See the full [`ROADMAP.md`](https://github.com/Rullst/Rullst/blob/main/rullst-iot/ROADMAP.md).

---

## 📄 License

Licensed under the [MIT license](https://github.com/Rullst/Rullst/blob/main/LICENSE).

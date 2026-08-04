# 🔌 rullst-iot Roadmap

> Dedicated development roadmap for the `rullst-iot` crate.

---

## 🏁 Phase 1: Core Telemetry & Protocol Drivers (Completed)
- [x] **`no_std` Compatibility:** Core data models and serializers operable in `no_std` environments.
- [x] **`SensorTelemetry` Data Structure:** Unified format for sensor metrics (temperature, vibration, pressure).
- [x] **MQTT / CoAP Protocol Helpers:** Payload formatters for edge network telemetry.
- [x] **CLI Scaffolding (`cargo rullst make:iot <DeviceName>`):** Single command generation for IoT node files.

---

## 📡 Phase 2: Edge Hardware Abstractions & Sensor HAL (Completed)
- [x] **Native GPIO & I2C Helpers:** Standardized cross-platform GPIO pin toggling (`GpioPin`) and I2C bus reading wrappers (`I2cHelper`).
- [x] **Modbus RTU / TCP Full Driver:** Direct industrial PLC communication driver with CRC-16 computation (`ModbusFrame`).
- [x] **BLE Telemetry GATT Server:** Custom GATT service generator (`GattService`, `GattCharacteristic`) for Bluetooth Low Energy beacons.

---

## 🧠 Phase 3: On-Device Edge AI & Local Inference (Completed)
- [x] **Micro-LLM & Sensor Anomaly Engine:** Embedded statistical anomaly detector (`AnomalyDetector`, `AnomalyState`) running locally without cloud internet dependencies.
- [x] **Embedded Micro-Dashboard (`rullst::iot::ui`):** HTMX-powered lightweight local web UI widget generator (`IotDashboard`).

---

## 🌐 Phase 4: Autonomous Edge Swarm & Mesh Networks (Completed)
- [x] **Zero-Config IoT Mesh Network (`rullst_iot::mesh`):** Self-healing P2P mesh network protocol (ESP-NOW / Thread / Zigbee) relaying telemetry between edge nodes when gateway link drops.
- [x] **Zero-Trust Over-The-Air (OTA) Updates (`rullst_iot::ota`):** Cryptographically signed (Ed25519) delta firmware update manager with dual-bank (A/B) bootloader rollback protection.

---

## 🛡️ Phase 5: Hardware Security & Cryptographic Enclaves (Completed)
- [x] **Hardware Security Element Bindings (`rullst_iot::hsm`):** Silicon bindings for hardware security chips (ATECC608A, TPM 2.0, STSAFE) storing private keys in tamper-proof hardware.
- [x] **Lightweight Post-Quantum Edge Encryption (`rullst_iot::pqc`):** Compacted NIST post-quantum (ML-KEM / Kyber) key exchange protecting low-power telemetry against quantum decryption.

---

## ⚡ Phase 6: Ultra-Low-Power & Digital Twin Engine (Completed)
- [x] **Deep Sleep & Power Governor (`rullst_iot::power`):** Dynamic power consumption governor managing Deep Sleep cycles, wake-on-interrupt triggers, and solar harvester voltage monitoring.
- [x] **Digital Twin Real-Time Sync (`rullst_iot::twin`):** Bi-directional real-time sync between hardware physical state (sensors, actuators) and cloud Digital Twin representation in Rullst Studio and Nexus.

---

## 🏭 Phase 7: Industrial Standards & Certification
- [ ] **OPC-UA Protocol Driver (`rullst_iot::opcua`):** Industrial OPC-UA driver for SCADA/MES/ERP communication in manufacturing plants — mandatory standard in Industry 4.0 (ISA-95, IEC 62541).
- [ ] **MQTT Sparkplug B Profile (`rullst_iot::sparkplug`):** IIoT-standard Sparkplug B profile over MQTT for interoperability with industrial platforms like Ignition, Unified Namespace, and AWS IoT.
- [ ] **IEC 61508 / IEC 62443 Safety Mode (`rullst_iot::safety`):** Deterministic execution mode with watchdog timer, stack overflow detection, and memory protection for SIL 2/3 safety-critical system certification.

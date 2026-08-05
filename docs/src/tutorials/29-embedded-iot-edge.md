# Tutorial 29: Embedded Systems & Edge IoT (`rullst-iot`) 🔌

Develop software for bare-metal microcontrollers (STM32, ESP32, Raspberry Pi) with `#![no_std]` support and under 2MB RAM footprint.

---

## 🛠️ Step 1: Scaffold an IoT Edge Device

```bash
cargo rullst make:iot SensorGateway
```

---

## 💻 Step 2: Read Hardware Registers & Run Edge AI Anomaly Detection

```rust
use rullst_iot::gpio::GpioPin;
use rullst_iot::anomaly::AnomalyDetector;

pub fn read_sensor_node() {
    let mut pin = GpioPin::new(14);
    pin.toggle();

    let mut detector = AnomalyDetector::new();
    let is_anomaly = detector.evaluate(98.6);

    if is_anomaly {
        println!("⚠️ Hardware anomaly detected on edge device!");
    }
}
```

---

## 💡 Key Takeaways
- `rullst-iot` supports Modbus, MQTT Sparkplug B, OPC-UA, BLE, and PQC encryption on `#![no_std]` targets.
- Verified against STM32 Cortex-M4 and ESP32-C3 RISC-V silicon.

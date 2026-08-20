// tests/iot_integration_tests.rs — Comprehensive unit and integration tests for Rullst IoT.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_iot::hsm::{HsmChipType, HsmDevice};
use rullst_iot::mesh::{MeshNode, MeshTopology, NodeStatus};
use rullst_iot::pqc::PqcKeyPair;
use rullst_iot::ui::IotDashboard;
use rullst_iot::*;

#[test]
fn test_anomaly_detector_and_thresholds() {
    let detector = AnomalyDetector::new(25.0, 5.0);
    assert_eq!(detector.evaluate(26.0), AnomalyState::Normal);
    assert_eq!(detector.evaluate(32.0), AnomalyState::Warning);
    assert_eq!(detector.evaluate(40.0), AnomalyState::CriticalAnomaly);
}

#[test]
fn test_modbus_frame_encoding() {
    let frame = ModbusFrame::read_holding_registers(1, 0, 10);
    assert_eq!(frame[0], 1);
    assert_eq!(frame[1], ModbusFunction::ReadHoldingRegisters as u8);
    assert_eq!(frame.len(), 8);

    let crc = ModbusFrame::calculate_crc16(&frame[..6]);
    assert_ne!(crc, 0);
}

#[test]
fn test_ota_partition_manager() {
    let mut ota = OtaManager::new("12.0.0");
    assert_eq!(ota.current_partition, BootPartition::PartitionA);
    assert_eq!(ota.current_partition.opposite(), BootPartition::PartitionB);

    let msg = ota.commit_update("12.1.0");
    assert_eq!(ota.current_partition, BootPartition::PartitionB);
    assert!(msg.contains("12.1.0"));

    assert!(ota.verify_signature(b"firmware_data", &[0u8; 64]));
    assert!(!ota.verify_signature(b"", &[0u8; 64]));
}

#[test]
fn test_digital_twin_and_power_governor() {
    let mut twin = DigitalTwin::new("sensor-edge-42");
    twin.ingest(SensorTelemetry::new(
        "sensor-edge-42",
        "temperature",
        24.5,
        1700000000,
    ));
    twin.ingest(SensorTelemetry::new(
        "sensor-edge-42",
        "humidity",
        60.0,
        1700000001,
    ));

    let latest_temp = twin.latest("temperature").unwrap();
    assert_eq!(latest_temp.value, 24.5);

    let sync_payload = twin.to_sync_payload();
    assert!(sync_payload.contains("sensor-edge-42"));

    let mut gov = PowerGovernor::new(2700, 3600);
    assert_eq!(gov.evaluate(), PowerMode::DeepSleep);
    assert_eq!(gov.harvester_state(), HarvesterState::Charging);

    gov.battery_mv = 4200;
    assert_eq!(gov.evaluate(), PowerMode::FullActive);
}

#[test]
fn test_gpio_and_ble_structures() {
    let mut pin = GpioPin::new(13, PinMode::Output);
    assert_eq!(pin.read(), PinState::Low);
    pin.set_high();
    assert_eq!(pin.read(), PinState::High);
    pin.set_low();
    assert_eq!(pin.read(), PinState::Low);

    let mut service = GattService::new("180F");
    service.add_characteristic("2A19", vec![100], true);
    assert_eq!(service.service_uuid, "180F");
    assert_eq!(service.characteristics.len(), 1);
    assert_eq!(service.characteristics[0].value[0], 100);
}

#[test]
fn test_hsm_device_and_signatures() {
    let hsm = HsmDevice::new(HsmChipType::Atecc608A, "DEV-SEC-001");
    let key = hsm.derive_key();
    assert_eq!(key.len(), 32);

    let sig = hsm.sign(b"telemetry_packet_bytes");
    assert_eq!(sig.len(), 32);
}

#[test]
fn test_pqc_keypair_encapsulation() {
    let keypair = PqcKeyPair::from_seed(b"pqc_master_seed_edge");
    assert_eq!(keypair.public_key.len(), 32);

    let ciphertext = keypair.encapsulate(b"shared_session_secret");
    assert_eq!(ciphertext.len(), 32);

    let decrypted = keypair.decapsulate(&ciphertext);
    assert_eq!(decrypted.len(), 32);
}

#[test]
fn test_mesh_topology_routing() {
    let mut mesh = MeshTopology::new();
    mesh.register(MeshNode::new("node_weak", -90));
    mesh.register(MeshNode::new("node_strong", -45));
    let mut offline = MeshNode::new("node_offline", -20);
    offline.status = NodeStatus::Offline;
    mesh.register(offline);

    let best = mesh.best_relay();
    assert!(best.is_some());
    assert_eq!(best.unwrap().node_id, "node_strong");
}

#[test]
fn test_iot_micro_dashboard_rendering() {
    let telemetry = SensorTelemetry::new("solar_node_1", "voltage", 3.3, 1700000000);
    let card_html = IotDashboard::render_sensor_card(&telemetry);
    assert!(card_html.contains("solar_node_1"));
    assert!(card_html.contains("3.3"));
    assert!(card_html.contains("voltage"));
    assert!(card_html.contains("ONLINE"));
}

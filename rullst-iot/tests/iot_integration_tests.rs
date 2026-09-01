// tests/iot_integration_tests.rs — Comprehensive unit and integration tests for Rullst IoT.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ed25519_dalek::{Signer, SigningKey};
#[cfg(feature = "experimental-simulators")]
use rullst_iot::hsm::{SimulatedHsmDevice, SimulatedHsmProfile};
use rullst_iot::mesh::{MeshNode, MeshTopology, NodeStatus};
#[cfg(feature = "experimental-simulators")]
use rullst_iot::pqc::SimulatedPqcFixture;
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
fn public_mqtt_publish_encoder_is_bounded_and_protocol_shaped() {
    let packet = MqttPublish::reliable(
        "factory/line-1/temperature",
        b"24.5".to_vec(),
        MqttQos::AtLeastOnce,
        9,
    )
    .unwrap()
    .encode()
    .unwrap();

    assert_eq!(packet[0], 0x32);
    assert!(packet.len() <= MAX_MQTT_PACKET_BYTES);
    assert!(packet.ends_with(b"24.5"));
}

#[test]
fn public_coap_request_encoder_orders_path_format_and_payload() {
    let packet = CoapRequest::new(
        CoapMessageType::Confirmable,
        CoapMethod::Post,
        42,
        [0x01, 0x02],
    )
    .unwrap()
    .path_segment("telemetry")
    .unwrap()
    .content_format(50)
    .payload(br#"{"temperature":24.5}"#.to_vec())
    .unwrap()
    .encode()
    .unwrap();

    assert_eq!(&packet[..6], &[0x42, 0x02, 0x00, 0x2a, 0x01, 0x02]);
    assert!(packet.contains(&0xff));
    assert!(packet.len() <= MAX_COAP_DATAGRAM_BYTES);
}

#[test]
fn test_ota_partition_manager() {
    let firmware = b"integration-test-firmware-v12.1.0";
    let signing_key = SigningKey::from_bytes(&[42_u8; 32]);
    let manifest =
        OtaManifest::from_firmware("rullst-test-board", "12.1.0", 121, firmware).unwrap();
    let signature = signing_key.sign(&manifest.signing_bytes().unwrap());
    let mut ota = OtaManager::new_with_trusted_key(
        "rullst-test-board",
        "12.0.0",
        120,
        signing_key.verifying_key().to_bytes(),
    )
    .unwrap();
    assert_eq!(ota.current_partition, BootPartition::PartitionA);
    assert_eq!(ota.current_partition.opposite(), BootPartition::PartitionB);

    assert_eq!(
        ota.commit_verified_update(),
        Err(OtaError::NoVerifiedUpdate)
    );
    ota.verify_update(&manifest, firmware, &signature.to_bytes())
        .unwrap();
    let commit = ota.commit_verified_update().unwrap();
    assert_eq!(ota.current_partition, BootPartition::PartitionB);
    assert_eq!(commit.version(), "12.1.0");
    assert_eq!(ota.rollback_counter(), 121);
    assert_eq!(
        ota.verify_update(&manifest, firmware, &signature.to_bytes()),
        Err(OtaError::RollbackDetected {
            current: 121,
            proposed: 121,
        })
    );
}

#[test]
// TM-IOT-01: forged signatures and any signed-field tampering fail closed.
fn test_ota_rejects_signature_hash_target_and_rollback_failures() {
    let firmware = b"firmware-image-A";
    let signing_key = SigningKey::from_bytes(&[17_u8; 32]);
    let public_key = signing_key.verifying_key().to_bytes();
    let manifest = OtaManifest::from_firmware("board-a", "2.0.0", 11, firmware).unwrap();
    let signature = signing_key
        .sign(&manifest.signing_bytes().unwrap())
        .to_bytes();
    let mut manager = OtaManager::new_with_trusted_key("board-a", "1.0.0", 10, public_key).unwrap();

    let mut invalid_signature = signature;
    invalid_signature[0] ^= 1;
    assert_eq!(
        manager.verify_update(&manifest, firmware, &invalid_signature),
        Err(OtaError::SignatureInvalid)
    );
    assert_eq!(
        manager.verify_update(&manifest, firmware, &[0_u8; 63]),
        Err(OtaError::InvalidSignatureEncoding)
    );
    assert_eq!(
        manager.verify_update(&manifest, b"short", &signature),
        Err(OtaError::FirmwareLengthMismatch {
            expected: 16,
            actual: 5,
        })
    );
    assert_eq!(
        manager.verify_update(&manifest, b"firmware-image-B", &signature),
        Err(OtaError::FirmwareHashMismatch)
    );

    let changed_version = OtaManifest::from_firmware("board-a", "2.0.1", 11, firmware).unwrap();
    assert_eq!(
        manager.verify_update(&changed_version, firmware, &signature),
        Err(OtaError::SignatureInvalid)
    );

    let wrong_target = OtaManifest::from_firmware("board-b", "2.0.0", 11, firmware).unwrap();
    let wrong_target_signature = signing_key
        .sign(&wrong_target.signing_bytes().unwrap())
        .to_bytes();
    assert_eq!(
        manager.verify_update(&wrong_target, firmware, &wrong_target_signature),
        Err(OtaError::TargetMismatch)
    );

    let rollback = OtaManifest::from_firmware("board-a", "0.9.0", 10, firmware).unwrap();
    let rollback_signature = signing_key
        .sign(&rollback.signing_bytes().unwrap())
        .to_bytes();
    assert_eq!(
        manager.verify_update(&rollback, firmware, &rollback_signature),
        Err(OtaError::RollbackDetected {
            current: 10,
            proposed: 10,
        })
    );
    assert!(manager.pending_manifest().is_none());
}

#[test]
#[allow(deprecated)]
fn test_legacy_ota_apis_are_typed_and_fail_closed() {
    assert!(matches!(
        OtaManager::new("1.0.0"),
        Err(OtaError::LegacyApiUnsupported {
            replacement: "OtaManager::new_with_trusted_key",
        })
    ));
    let signing_key = SigningKey::from_bytes(&[23_u8; 32]);
    let mut manager = OtaManager::new_with_trusted_key(
        "legacy-test-board",
        "1.0.0",
        1,
        signing_key.verifying_key().to_bytes(),
    )
    .unwrap();
    assert_eq!(
        manager.verify_signature(b"firmware", &[0_u8; 64]),
        Err(OtaError::LegacyApiUnsupported {
            replacement: "OtaManager::verify_update",
        })
    );
    assert_eq!(
        manager.commit_update("2.0.0"),
        Err(OtaError::LegacyApiUnsupported {
            replacement: "OtaManager::commit_verified_update",
        })
    );
    assert_eq!(manager.current_partition, BootPartition::PartitionA);
    assert_eq!(manager.firmware_version, "1.0.0");
}

#[test]
fn test_ota_rejects_invalid_trust_configuration() {
    let mut weak_identity_key = [0_u8; 32];
    weak_identity_key[0] = 1;
    assert_eq!(
        OtaManager::new_with_trusted_key("board-a", "1.0.0", 1, weak_identity_key).err(),
        Some(OtaError::InvalidTrustedKey)
    );
    assert_eq!(
        OtaManifest::from_firmware("", "2.0.0", 2, b"firmware"),
        Err(OtaError::EmptyTarget)
    );
    assert_eq!(
        OtaManifest::from_firmware("board-a", "", 2, b"firmware"),
        Err(OtaError::EmptyVersion)
    );
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
#[cfg(feature = "experimental-simulators")]
fn test_simulated_hsm_fixture_digests() {
    let hsm = SimulatedHsmDevice::new(SimulatedHsmProfile::Atecc608A, "DEV-SEC-001");
    let fixture = hsm.derive_fixture_bytes();
    assert_eq!(fixture.len(), 32);

    let digest = hsm.digest_fixture(b"telemetry_packet_bytes");
    assert_eq!(digest.len(), 32);
}

#[test]
#[cfg(feature = "experimental-simulators")]
fn test_simulated_pqc_fixture_derivations() {
    let fixture = SimulatedPqcFixture::from_seed(b"pqc_fixture_seed_edge");
    assert_eq!(fixture.public_fixture().len(), 32);

    let ciphertext = fixture.derive_ciphertext_fixture(b"fixture_input");
    assert_eq!(ciphertext.len(), 32);

    let output = fixture.derive_output_fixture(&ciphertext);
    assert_eq!(output.len(), 32);
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
    assert!(card_html.contains("SNAPSHOT"));
    assert!(!card_html.contains("ONLINE"));
}

#[test]
#[cfg(feature = "experimental-simulators")]
fn test_simulated_hsm_profiles() {
    let hsm_tpm = SimulatedHsmDevice::new(SimulatedHsmProfile::Tpm2, "DEV-TPM-002");
    assert_eq!(hsm_tpm.derive_fixture_bytes().len(), 32);

    let hsm_stsafe = SimulatedHsmDevice::new(SimulatedHsmProfile::Stsafe, "DEV-SE-003");
    assert_eq!(hsm_stsafe.derive_fixture_bytes().len(), 32);

    let hsm_software = SimulatedHsmDevice::new(SimulatedHsmProfile::Software, "DEV-CUST-004");
    assert_eq!(hsm_software.derive_fixture_bytes().len(), 32);
}

#[test]
fn test_power_governor_modes() {
    let mut gov = PowerGovernor::new(2500, 3300);
    gov.battery_mv = 2400;
    assert_eq!(gov.evaluate(), PowerMode::DeepSleep);

    gov.battery_mv = 3000;
    assert_eq!(gov.evaluate(), PowerMode::LowPower);

    gov.battery_mv = 3500;
    assert_eq!(gov.evaluate(), PowerMode::FullActive);
}

#[test]
fn test_i2c_read_frames() {
    let frame = rullst_iot::i2c::I2cHelper::build_read_frame(0x50, 0x00, 4);
    assert_eq!(frame[0], 0xA0);
    assert_eq!(frame[1], 0x00);
    assert_eq!(frame[2], 0xA1);
    assert_eq!(frame.len(), 7);
}

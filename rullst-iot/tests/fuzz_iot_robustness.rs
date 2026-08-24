use ed25519_dalek::SigningKey;
use rullst_iot::SensorTelemetry;
use rullst_iot::anomaly::AnomalyDetector;
use rullst_iot::modbus::ModbusFrame;
use rullst_iot::ota::{OtaManager, OtaManifest};
#[cfg(feature = "experimental-simulators")]
use rullst_iot::pqc::SimulatedPqcFixture;
use rullst_iot::twin::DigitalTwin;

#[test]
fn test_fuzz_modbus_crc_zero_panics() {
    let mut rng_seed: u64 = 0xcafe_1234_5678;
    let mut lcg = || {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (rng_seed >> 32) as u32
    };

    for _ in 0..5_000 {
        let len = (lcg() % 256) as usize;
        let mut buf = vec![0u8; len];
        for b in &mut buf {
            *b = (lcg() % 256) as u8;
        }

        // Must never panic on arbitrary byte sequences
        let _ = ModbusFrame::calculate_crc16(&buf);
    }
}

#[test]
fn test_fuzz_anomaly_detector_floats() {
    let mut rng_seed: u64 = 0x9876_5432_10fe;
    let mut lcg = || {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (rng_seed >> 32) as u32
    };

    let special_floats = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN,
        f64::MAX,
        f64::MIN_POSITIVE,
        0.0,
        -0.0,
        1e-300,
        1e300,
    ];

    let detector = AnomalyDetector::new(50.0, 10.0);

    for &val in &special_floats {
        let _ = detector.evaluate(val);
    }

    for _ in 0..5_000 {
        let bits = ((lcg() as u64) << 32) | (lcg() as u64);
        let val = f64::from_bits(bits);
        let _ = detector.evaluate(val);
    }
}

#[test]
fn test_fuzz_digital_twin_and_ota() {
    let mut twin = DigitalTwin::new("fuzz-sensor-01");

    let mut rng_seed: u64 = 0x1122_3344_5566;
    let mut lcg = || {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (rng_seed >> 32) as u32
    };

    for i in 0..2_000 {
        let metric = match lcg() % 4 {
            0 => "temperature",
            1 => "vibration",
            2 => "humidity",
            _ => "pressure",
        };
        let val = (lcg() % 1000) as f64 / 10.0;
        twin.ingest(SensorTelemetry::new(
            "fuzz-sensor-01",
            metric,
            val,
            1700000000 + i,
        ));
    }

    assert!(twin.latest("temperature").is_some());

    let signing_key = SigningKey::from_bytes(&[31_u8; 32]);
    let mut ota = OtaManager::new_with_trusted_key(
        "fuzz-board",
        "12.0.0",
        12,
        signing_key.verifying_key().to_bytes(),
    )
    .unwrap();
    for counter in 13_u64..141 {
        let firmware = counter.to_be_bytes();
        let manifest =
            OtaManifest::from_firmware("fuzz-board", "next", counter, &firmware).unwrap();
        let mut invalid_signature = [0_u8; 64];
        for byte in &mut invalid_signature {
            *byte = (lcg() % 256) as u8;
        }
        assert!(
            ota.verify_update(&manifest, &firmware, &invalid_signature)
                .is_err()
        );
    }
}

#[test]
#[cfg(feature = "experimental-simulators")]
fn test_fuzz_simulated_pqc_fixture_invariants() {
    let fixture = SimulatedPqcFixture::from_seed(b"fuzz_pqc_fixture_sensor_01");
    assert!(!fixture.public_fixture().is_empty());

    let ciphertext = fixture.derive_ciphertext_fixture(b"sensor_telemetry_payload");
    assert!(!ciphertext.is_empty());

    let output = fixture.derive_output_fixture(&ciphertext);
    assert!(!output.is_empty());
}

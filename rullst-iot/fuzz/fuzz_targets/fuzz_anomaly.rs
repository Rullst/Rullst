#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_iot::anomaly::AnomalyDetector;

fuzz_target!(|data: &[u8]| {
    if data.len() >= 24 {
        let mean = f64::from_le_bytes(data[0..8].try_into().unwrap());
        let tol = f64::from_le_bytes(data[8..16].try_into().unwrap());
        let val = f64::from_le_bytes(data[16..24].try_into().unwrap());
        if !mean.is_nan() && !tol.is_nan() && !val.is_nan() && tol > 0.0 {
            let detector = AnomalyDetector::new(mean, tol);
            let _ = detector.evaluate(val);
        }
    }
});

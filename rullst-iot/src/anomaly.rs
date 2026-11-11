//! `no_std` statistical threshold classification for sensor readings.

use core::fmt;
use serde::{Deserialize, Serialize};

/// Classification result for sensor readings.
///
/// It serializes as `"normal"`, `"warning"` or `"critical_anomaly"`, and
/// displays as `normal`, `warning` or `critical anomaly` (13.0.0-alpha.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyState {
    /// Within one tolerance of the expected mean (inclusive).
    Normal,
    /// More than one and at most two tolerances from the expected mean.
    Warning,
    /// More than two tolerances from the mean, or a non-finite reading or
    /// configuration, which fails closed here.
    CriticalAnomaly,
}

impl AnomalyState {
    /// The display text: `normal`, `warning` or `critical anomaly`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Warning => "warning",
            Self::CriticalAnomaly => "critical anomaly",
        }
    }
}

impl fmt::Display for AnomalyState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Lightweight statistical anomaly detector.
///
/// `Clone`, `Copy`, `Debug` and `PartialEq` since 13.0.0-alpha.5.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnomalyDetector {
    expected_mean: f64,
    allowed_tolerance: f64,
}

impl AnomalyDetector {
    /// Creates an anomaly detector with an expected baseline mean and tolerance.
    pub fn new(expected_mean: f64, allowed_tolerance: f64) -> Self {
        Self {
            expected_mean,
            allowed_tolerance,
        }
    }

    /// Evaluates a raw sensor value against baseline expectations.
    ///
    /// Non-finite readings or configuration values fail closed as critical
    /// anomalies instead of being silently classified as normal.
    pub fn evaluate(&self, value: f64) -> AnomalyState {
        if !value.is_finite()
            || !self.expected_mean.is_finite()
            || !self.allowed_tolerance.is_finite()
            || self.allowed_tolerance < 0.0
        {
            return AnomalyState::CriticalAnomaly;
        }

        let diff = (value - self.expected_mean).abs();
        if diff > (self.allowed_tolerance * 2.0) {
            AnomalyState::CriticalAnomaly
        } else if diff > self.allowed_tolerance {
            AnomalyState::Warning
        } else {
            AnomalyState::Normal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_display_and_serialize_as_documented() {
        use alloc::string::ToString;
        for (state, text, json) in [
            (AnomalyState::Normal, "normal", "\"normal\""),
            (AnomalyState::Warning, "warning", "\"warning\""),
            (
                AnomalyState::CriticalAnomaly,
                "critical anomaly",
                "\"critical_anomaly\"",
            ),
        ] {
            assert_eq!(state.to_string(), text);
            assert_eq!(state.as_str(), text);
            assert_eq!(serde_json::to_string(&state).unwrap(), json);
            assert_eq!(serde_json::from_str::<AnomalyState>(json).unwrap(), state);
        }
        assert!(serde_json::from_str::<AnomalyState>("\"critical\"").is_err());
    }

    #[test]
    fn detectors_compare_copy_and_debug() {
        let detector = AnomalyDetector::new(25.0, 5.0);
        let copy = detector;
        assert_eq!(copy, detector.clone());
        assert_ne!(copy, AnomalyDetector::new(25.0, 4.0));
        assert_eq!(
            alloc::format!("{detector:?}"),
            "AnomalyDetector { expected_mean: 25.0, allowed_tolerance: 5.0 }"
        );
    }

    #[test]
    fn test_anomaly_detection() {
        let detector = AnomalyDetector::new(25.0, 5.0);
        assert_eq!(detector.evaluate(26.0), AnomalyState::Normal);
        assert_eq!(detector.evaluate(32.0), AnomalyState::Warning);
        assert_eq!(detector.evaluate(40.0), AnomalyState::CriticalAnomaly);
    }

    #[test]
    fn invalid_readings_and_configuration_fail_closed() {
        let detector = AnomalyDetector::new(25.0, 5.0);
        assert_eq!(detector.evaluate(f64::NAN), AnomalyState::CriticalAnomaly);
        assert_eq!(
            detector.evaluate(f64::INFINITY),
            AnomalyState::CriticalAnomaly
        );

        let invalid_tolerance = AnomalyDetector::new(25.0, -1.0);
        assert_eq!(
            invalid_tolerance.evaluate(25.0),
            AnomalyState::CriticalAnomaly
        );
        let invalid_baseline = AnomalyDetector::new(f64::NAN, 1.0);
        assert_eq!(
            invalid_baseline.evaluate(25.0),
            AnomalyState::CriticalAnomaly
        );
        let unbounded_tolerance = AnomalyDetector::new(25.0, f64::INFINITY);
        assert_eq!(
            unbounded_tolerance.evaluate(25.0),
            AnomalyState::CriticalAnomaly
        );
    }

    #[test]
    fn thresholds_are_inclusive_at_one_and_two_tolerances() {
        let detector = AnomalyDetector::new(25.0, 5.0);
        assert_eq!(detector.evaluate(30.0), AnomalyState::Normal);
        assert_eq!(detector.evaluate(33.0), AnomalyState::Warning);
        assert_eq!(detector.evaluate(35.0), AnomalyState::Warning);
        assert_eq!(detector.evaluate(15.0), AnomalyState::Warning);
        assert_eq!(detector.evaluate(35.5), AnomalyState::CriticalAnomaly);

        let exact = AnomalyDetector::new(25.0, 0.0);
        assert_eq!(exact.evaluate(25.0), AnomalyState::Normal);
        assert_eq!(exact.evaluate(25.5), AnomalyState::CriticalAnomaly);
    }
}

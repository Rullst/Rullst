//! `no_std` statistical threshold classification for sensor readings.

/// Classification result for sensor readings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnomalyState {
    Normal,
    Warning,
    CriticalAnomaly,
}

/// Lightweight statistical anomaly detector.
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
    }
}

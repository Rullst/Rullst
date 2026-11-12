//! A Software Carbon Intensity estimate (ISO/IEC 21031:2024):
//! `SCI = ((E × I) + M) / R`. Footprint measures no energy, so E is always an
//! estimate: the app's measured CPU time × the power figure you supply, plus,
//! when you supply an energy figure per gigabyte, the measured response bytes
//! × that figure. I and M are yours too. Nothing is fetched. The result is
//! labelled `estimate` with every input; it is never a measurement.
use serde::Serialize;

pub(super) const LABEL: &str = "estimate";
pub(super) const STANDARD: &str = "Software Carbon Intensity (SCI), ISO/IEC 21031:2024";
pub(super) const FORMULA: &str = "SCI = ((E × I) + M) / R";
pub(super) const ENERGY_METHOD: &str = "E = measured app CPU time × --cpu-watts (your power figure per busy CPU second), plus the network term when --network-kwh-per-gb is given";
pub(super) const NETWORK_METHOD: &str = "E_network = measured response body bytes ÷ 10^9 × --network-kwh-per-gb (your energy figure per gigabyte transferred)";
pub(super) const FUNCTIONAL_UNIT: &str =
    "one HTTP request; R = requests completed in the benchmark";
const JOULES_PER_KWH: f64 = 3_600_000.0;
/// Bytes per gigabyte, as energy-per-GB figures count them (10^9).
const BYTES_PER_GB: f64 = 1e9;

/// Every input of the estimate and where it came from.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct Inputs {
    /// Measured during the benchmark.
    pub cpu_time_seconds: Option<f64>,
    /// Supplied with `--cpu-watts`.
    pub cpu_watts: Option<f64>,
    /// Supplied with `--grid-intensity`.
    pub grid_intensity_g_per_kwh: Option<f64>,
    /// Supplied with `--embodied`; not included when absent.
    pub embodied_g: Option<f64>,
    /// Measured during the benchmark.
    pub requests: Option<u64>,
    /// Supplied with `--network-kwh-per-gb`; no network term when absent.
    pub network_kwh_per_gb: Option<f64>,
    /// Response body bytes measured during the benchmark.
    pub response_bytes: Option<u64>,
}

/// The estimate, or which inputs were missing.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct SciEstimate {
    /// `estimate` when computed, otherwise `not_computed`.
    pub status: &'static str,
    /// Always `estimate`: this figure is never a measurement.
    pub label: &'static str,
    pub standard: &'static str,
    pub formula: &'static str,
    pub energy_method: &'static str,
    pub network_method: &'static str,
    pub functional_unit: &'static str,
    pub inputs: Inputs,
    /// E: the CPU term plus the network term when it is included.
    pub energy_kwh: Option<f64>,
    pub cpu_energy_kwh: Option<f64>,
    /// `null` without `--network-kwh-per-gb`.
    pub network_energy_kwh: Option<f64>,
    pub sci_g_per_request: Option<f64>,
    /// Inputs that prevented the estimate.
    pub missing_inputs: Vec<&'static str>,
}

pub(super) fn estimate(inputs: Inputs) -> SciEstimate {
    let mut missing_inputs = Vec::new();
    if inputs.cpu_time_seconds.is_none() {
        missing_inputs.push("cpu_time_seconds");
    }
    if inputs.cpu_watts.is_none() {
        missing_inputs.push("cpu_watts");
    }
    if inputs.grid_intensity_g_per_kwh.is_none() {
        missing_inputs.push("grid_intensity_g_per_kwh");
    }
    if inputs.requests.is_none_or(|requests| requests == 0) {
        missing_inputs.push("requests");
    }
    if inputs.network_kwh_per_gb.is_some() && inputs.response_bytes.is_none() {
        missing_inputs.push("response_bytes");
    }
    let cpu_energy_kwh = match (inputs.cpu_time_seconds, inputs.cpu_watts) {
        (Some(seconds), Some(watts)) => Some(seconds * watts / JOULES_PER_KWH),
        _ => None,
    };
    let network_energy_kwh = match (inputs.response_bytes, inputs.network_kwh_per_gb) {
        (Some(bytes), Some(per_gb)) => Some(bytes as f64 / BYTES_PER_GB * per_gb),
        _ => None,
    };
    let energy_kwh = match (cpu_energy_kwh, inputs.network_kwh_per_gb) {
        (Some(cpu), None) => Some(cpu),
        (Some(cpu), Some(_)) => network_energy_kwh.map(|network| cpu + network),
        (None, _) => None,
    };
    let sci_g_per_request = match (energy_kwh, inputs.grid_intensity_g_per_kwh, inputs.requests) {
        (Some(energy), Some(intensity), Some(requests)) if requests > 0 => {
            Some((energy * intensity + inputs.embodied_g.unwrap_or(0.0)) / requests as f64)
        }
        _ => None,
    };
    SciEstimate {
        status: if sci_g_per_request.is_some() {
            "estimate"
        } else {
            "not_computed"
        },
        label: LABEL,
        standard: STANDARD,
        formula: FORMULA,
        energy_method: ENERGY_METHOD,
        network_method: NETWORK_METHOD,
        functional_unit: FUNCTIONAL_UNIT,
        inputs,
        energy_kwh,
        cpu_energy_kwh,
        network_energy_kwh,
        sci_g_per_request,
        missing_inputs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: Option<f64>, right: f64) -> bool {
        left.is_some_and(|left| (left - right).abs() < 1e-12)
    }

    fn full() -> Inputs {
        Inputs {
            cpu_time_seconds: Some(36.0),
            cpu_watts: Some(100.0),
            grid_intensity_g_per_kwh: Some(400.0),
            embodied_g: None,
            requests: Some(1_000),
            network_kwh_per_gb: None,
            response_bytes: None,
        }
    }

    #[test]
    fn supplied_inputs_give_a_labelled_estimate_per_request() {
        // 36 s × 100 W = 3600 J = 0.001 kWh; × 400 g/kWh = 0.4 g over 1000 requests.
        let estimate = estimate(full());
        assert_eq!(estimate.status, "estimate");
        assert_eq!(estimate.label, "estimate");
        assert!(close(estimate.energy_kwh, 0.001));
        assert!(close(estimate.sci_g_per_request, 0.0004));
        assert!(estimate.missing_inputs.is_empty());
        assert_eq!(estimate.inputs, full());

        assert!(close(estimate.cpu_energy_kwh, 0.001));
        assert_eq!(estimate.network_energy_kwh, None);

        let with_embodied = super::estimate(Inputs {
            embodied_g: Some(0.2),
            ..full()
        });
        assert!(close(with_embodied.sci_g_per_request, 0.0006));
    }

    #[test]
    fn the_network_term_adds_the_energy_of_the_bytes_served() {
        // 2.5 × 10^9 bytes × 0.4 kWh/GB = 1 kWh, plus 0.001 kWh of CPU; ×
        // 400 g/kWh = 400.4 g over 1000 requests.
        let estimate = estimate(Inputs {
            network_kwh_per_gb: Some(0.4),
            response_bytes: Some(2_500_000_000),
            ..full()
        });
        assert_eq!(estimate.status, "estimate");
        assert!(close(estimate.cpu_energy_kwh, 0.001));
        assert!(close(estimate.network_energy_kwh, 1.0));
        assert!(close(estimate.energy_kwh, 1.001));
        assert!(close(estimate.sci_g_per_request, 0.4004));
        assert!(estimate.missing_inputs.is_empty());
        assert_eq!(estimate.network_method, NETWORK_METHOD);

        // Bytes without a figure per gigabyte add nothing.
        let unused = super::estimate(Inputs {
            response_bytes: Some(2_500_000_000),
            ..full()
        });
        assert!(close(unused.energy_kwh, 0.001));
        assert_eq!(unused.network_energy_kwh, None);

        // A figure per gigabyte without measured bytes stops the estimate.
        let unmeasured = super::estimate(Inputs {
            network_kwh_per_gb: Some(0.4),
            ..full()
        });
        assert_eq!(unmeasured.status, "not_computed");
        assert_eq!(unmeasured.missing_inputs, ["response_bytes"]);
        assert_eq!(
            (unmeasured.energy_kwh, unmeasured.sci_g_per_request),
            (None, None)
        );
        assert!(close(unmeasured.cpu_energy_kwh, 0.001));

        let no_cpu = super::estimate(Inputs {
            cpu_time_seconds: None,
            network_kwh_per_gb: Some(0.4),
            response_bytes: Some(1),
            ..full()
        });
        assert_eq!(no_cpu.energy_kwh, None);
        assert!(close(no_cpu.network_energy_kwh, 4e-10));
    }

    #[test]
    fn missing_inputs_are_named_and_nothing_is_estimated() {
        let none = estimate(Inputs::default());
        assert_eq!(none.status, "not_computed");
        assert_eq!(none.label, "estimate");
        assert_eq!(
            none.missing_inputs,
            [
                "cpu_time_seconds",
                "cpu_watts",
                "grid_intensity_g_per_kwh",
                "requests"
            ]
        );
        assert_eq!((none.energy_kwh, none.sci_g_per_request), (None, None));

        let no_cpu = estimate(Inputs {
            cpu_time_seconds: None,
            ..full()
        });
        assert_eq!(no_cpu.missing_inputs, ["cpu_time_seconds"]);
        assert_eq!(no_cpu.energy_kwh, None);

        // Energy and intensity without a completed request: no division by zero.
        let no_requests = estimate(Inputs {
            requests: Some(0),
            ..full()
        });
        assert_eq!(no_requests.status, "not_computed");
        assert_eq!(no_requests.missing_inputs, ["requests"]);
        assert!(close(no_requests.energy_kwh, 0.001));
        assert_eq!(no_requests.sci_g_per_request, None);

        let no_intensity = estimate(Inputs {
            grid_intensity_g_per_kwh: None,
            ..full()
        });
        assert_eq!(no_intensity.missing_inputs, ["grid_intensity_g_per_kwh"]);
        assert_eq!(no_intensity.sci_g_per_request, None);
    }
}

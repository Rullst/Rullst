//! A Software Carbon Intensity estimate (ISO/IEC 21031:2024):
//! `SCI = ((E × I) + M) / R`. Green measures no energy, so E is always an
//! estimate (the app's measured CPU time × the power figure you supply), and
//! I and M are yours too. Nothing is fetched. The result is labelled
//! `estimate` with every input; it is never a measurement.
use serde::Serialize;

pub(super) const LABEL: &str = "estimate";
pub(super) const STANDARD: &str = "Software Carbon Intensity (SCI), ISO/IEC 21031:2024";
pub(super) const FORMULA: &str = "SCI = ((E × I) + M) / R";
pub(super) const ENERGY_METHOD: &str =
    "E = measured app CPU time × --cpu-watts (your power figure per busy CPU second)";
pub(super) const FUNCTIONAL_UNIT: &str =
    "one HTTP request; R = requests completed in the benchmark";
const JOULES_PER_KWH: f64 = 3_600_000.0;

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
    pub functional_unit: &'static str,
    pub inputs: Inputs,
    pub energy_kwh: Option<f64>,
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
    let energy_kwh = match (inputs.cpu_time_seconds, inputs.cpu_watts) {
        (Some(seconds), Some(watts)) => Some(seconds * watts / JOULES_PER_KWH),
        _ => None,
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
        functional_unit: FUNCTIONAL_UNIT,
        inputs,
        energy_kwh,
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

        let with_embodied = super::estimate(Inputs {
            embodied_g: Some(0.2),
            ..full()
        });
        assert!(close(with_embodied.sci_g_per_request, 0.0006));
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

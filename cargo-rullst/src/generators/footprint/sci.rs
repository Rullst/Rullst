//! Software Carbon Intensity (ISO/IEC 21031:2024): `SCI = ((E × I) + M) / R`,
//! computed only from a known E and a user-provided I. Nothing is fetched.
use super::energy::{Energy, kwh};
use serde::Serialize;

pub(super) const STANDARD: &str = "Software Carbon Intensity (SCI), ISO/IEC 21031:2024";
pub(super) const FORMULA: &str = "SCI = ((E × I) + M) / R";
pub(super) const FUNCTIONAL_UNIT: &str =
    "one HTTP request; R = requests completed during the load window";

/// Every SCI term with its source, and the result when it can be computed.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Carbon {
    pub status: &'static str,
    pub standard: &'static str,
    pub formula: &'static str,
    pub functional_unit: &'static str,
    pub e_kwh: Option<f64>,
    pub e_source: String,
    pub i_g_per_kwh: Option<f64>,
    pub i_source: &'static str,
    pub m_g: Option<f64>,
    pub m_source: &'static str,
    pub r_requests: u64,
    pub operational_g: Option<f64>,
    pub total_g: Option<f64>,
    pub sci_g_per_request: Option<f64>,
    /// Terms whose value is an estimate rather than a measurement.
    pub estimated_terms: Vec<&'static str>,
    /// Terms that prevented the computation.
    pub missing_terms: Vec<&'static str>,
}

pub(super) fn compute(
    energy: &Energy,
    grid_intensity: Option<f64>,
    embodied: Option<f64>,
    requests: u64,
) -> Carbon {
    let e_kwh = energy.joules().map(kwh);
    let e_source = match energy {
        Energy::Measured { method, .. } | Energy::Estimate { method, .. } => method.clone(),
        Energy::NotMeasured { .. } => "not measured".to_string(),
    };
    let mut missing_terms = Vec::new();
    if e_kwh.is_none() {
        missing_terms.push("E");
    }
    if grid_intensity.is_none() {
        missing_terms.push("I");
    }
    if requests == 0 {
        missing_terms.push("R");
    }
    let estimated_terms = if energy.is_estimate() {
        vec!["E"]
    } else {
        Vec::new()
    };
    let (operational_g, total_g, sci_g_per_request) = match (e_kwh, grid_intensity) {
        (Some(e_kwh), Some(intensity)) if requests > 0 => {
            let operational = e_kwh * intensity;
            let total = operational + embodied.unwrap_or(0.0);
            (
                Some(operational),
                Some(total),
                Some(total / requests as f64),
            )
        }
        _ => (None, None, None),
    };
    Carbon {
        status: if sci_g_per_request.is_some() {
            "computed"
        } else {
            "not_computed"
        },
        standard: STANDARD,
        formula: FORMULA,
        functional_unit: FUNCTIONAL_UNIT,
        e_kwh,
        e_source,
        i_g_per_kwh: grid_intensity,
        i_source: if grid_intensity.is_some() {
            "user-provided (--grid-intensity)"
        } else {
            "not provided (pass --grid-intensity from your own source)"
        },
        m_g: embodied,
        m_source: if embodied.is_some() {
            "user-provided (--embodied)"
        } else {
            "not included"
        },
        r_requests: requests,
        operational_g,
        total_g,
        sci_g_per_request,
        estimated_terms,
        missing_terms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: Option<f64>, right: f64) -> bool {
        left.is_some_and(|left| (left - right).abs() < 1e-12)
    }

    #[test]
    fn measured_energy_and_intensity_give_sci_per_request() {
        let energy = Energy::decide(Ok((3_600.0, vec!["package-0".into()])), None, None);
        let carbon = compute(&energy, Some(400.0), Some(0.2), 1_000);
        assert_eq!(carbon.status, "computed");
        assert!(close(carbon.e_kwh, 0.001));
        assert!(close(carbon.operational_g, 0.4));
        assert!(close(carbon.total_g, 0.6));
        assert!(close(carbon.sci_g_per_request, 0.0006));
        assert!(carbon.estimated_terms.is_empty());
        assert!(carbon.missing_terms.is_empty());
        assert!(carbon.e_source.starts_with("measured (RAPL"));
        assert_eq!(carbon.i_source, "user-provided (--grid-intensity)");
        assert_eq!(carbon.m_source, "user-provided (--embodied)");
    }

    #[test]
    fn estimated_energy_is_labelled_and_embodied_is_optional() {
        let energy = Energy::decide(Err("denied".into()), Some(36.0), Some(100.0));
        let carbon = compute(&energy, Some(100.0), None, 10);
        assert_eq!(carbon.status, "computed");
        assert_eq!(carbon.estimated_terms, ["E"]);
        assert!(carbon.e_source.starts_with("estimate:"));
        assert_eq!(carbon.m_g, None);
        assert_eq!(carbon.m_source, "not included");
        // 3600 J = 0.001 kWh; 0.1 g over 10 requests.
        assert!(close(carbon.sci_g_per_request, 0.01));
        assert_eq!(carbon.operational_g, carbon.total_g);
    }

    #[test]
    fn missing_inputs_are_named_and_nothing_is_computed() {
        let none = Energy::decide(Err("denied".into()), None, None);
        let carbon = compute(&none, None, Some(1.0), 0);
        assert_eq!(carbon.status, "not_computed");
        assert_eq!(carbon.missing_terms, ["E", "I", "R"]);
        assert_eq!(carbon.e_source, "not measured");
        assert_eq!(
            (
                carbon.operational_g,
                carbon.total_g,
                carbon.sci_g_per_request
            ),
            (None, None, None)
        );

        let measured = Energy::decide(Ok((1.0, Vec::new())), None, None);
        let without_intensity = compute(&measured, None, None, 5);
        assert_eq!(without_intensity.missing_terms, ["I"]);
        assert!(without_intensity.i_source.starts_with("not provided"));
        assert_eq!(without_intensity.sci_g_per_request, None);

        // Energy and intensity without a request leave R missing: no
        // division by zero.
        let without_requests = compute(&measured, Some(400.0), None, 0);
        assert_eq!(without_requests.status, "not_computed");
        assert_eq!(without_requests.missing_terms, ["R"]);
        assert_eq!(
            (
                without_requests.operational_g,
                without_requests.sci_g_per_request
            ),
            (None, None)
        );
    }
}

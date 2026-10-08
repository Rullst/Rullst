//! JSON key stability and terminal rendering of a fixed report.
use super::energy::Energy;
use super::load::Outcome;
use super::report::*;
use super::*;
use std::time::Duration;

fn sample(energy: &Energy, grid_intensity: Option<f64>) -> Report {
    let outcome = Outcome {
        responses: 400,
        transport_errors: 1,
        http_errors: 2,
        elapsed: Duration::from_secs(2),
        latencies_us: (1..=400).map(|value| value * 10).collect(),
    };
    Report {
        schema_version: SCHEMA_VERSION,
        cli_version: "13.0.0-test",
        measured_at: "2026-10-08T12:00:00Z".to_string(),
        machine: Machine {
            os: "linux",
            arch: "x86_64",
            logical_cpus: Some(4),
            cpu_model: Some("Example CPU".to_string()),
        },
        inputs: Inputs {
            url: None,
            path: "/".to_string(),
            duration_seconds: 2.0,
            concurrency: 4,
            grid_intensity_g_per_kwh: grid_intensity,
            cpu_watts: Some(15.0),
            embodied_g: None,
        },
        target: Target {
            mode: "release_build",
            url: "http://127.0.0.1:41234/".to_string(),
            pid: Some(4242),
            process_lookup: "started by footprint".to_string(),
            executable: Some("app".to_string()),
            environment: Some("RULLST_ENV=production HOST=127.0.0.1"),
        },
        load: Load::from_outcome(&outcome),
        process: Process {
            cpu_time_seconds: Metric::measured(1.5, "seconds", "utime + stime"),
            peak_rss_bytes: Metric::measured(18 * 1024 * 1024, "bytes", "VmHWM"),
            idle_rss_bytes: Metric::not_measured("bytes", "not readable"),
        },
        artifacts: Artifacts {
            binary_size_bytes: Metric::measured(8 * 1024 * 1024, "bytes", "file size"),
            docker_image_size_bytes: Metric::not_measured("bytes", "no local image"),
        },
        energy: EnergyView::new(energy, outcome.responses),
        carbon: sci::compute(energy, grid_intensity, None, outcome.responses),
        notes: NOTES,
    }
}

/// Dotted paths of every leaf value.
fn key_paths(value: &serde_json::Value, prefix: &str, paths: &mut Vec<String>) {
    let serde_json::Value::Object(map) = value else {
        paths.push(prefix.to_string());
        return;
    };
    for (key, child) in map {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        key_paths(child, &path, paths);
    }
}

#[test]
fn json_keys_are_stable_whatever_was_measured() {
    let estimate = Energy::decide(Err("denied".into()), Some(1.5), Some(15.0));
    let missing = Energy::decide(Err("denied".into()), None, None);
    let mut shapes = Vec::new();
    for report in [sample(&estimate, Some(100.0)), sample(&missing, None)] {
        let value = serde_json::to_value(&report).unwrap();
        let mut paths = Vec::new();
        key_paths(&value, "", &mut paths);
        paths.sort();
        shapes.push(paths);
    }
    assert_eq!(
        shapes[0], shapes[1],
        "the key set does not depend on values"
    );
    let mut expected: Vec<String> = [
        "carbon.e_kwh",
        "carbon.e_source",
        "carbon.estimated_terms",
        "carbon.formula",
        "carbon.functional_unit",
        "carbon.i_g_per_kwh",
        "carbon.i_source",
        "carbon.m_g",
        "carbon.m_source",
        "carbon.missing_terms",
        "carbon.operational_g",
        "carbon.r_requests",
        "carbon.sci_g_per_request",
        "carbon.standard",
        "carbon.status",
        "carbon.total_g",
        "cli_version",
        "energy.cpu_seconds",
        "energy.cpu_watts",
        "energy.joules",
        "energy.joules_per_request",
        "energy.kwh",
        "energy.method",
        "energy.rapl_zones",
        "energy.reason",
        "energy.status",
        "inputs.concurrency",
        "inputs.cpu_watts",
        "inputs.duration_seconds",
        "inputs.embodied_g",
        "inputs.grid_intensity_g_per_kwh",
        "inputs.path",
        "inputs.url",
        "load.elapsed_seconds",
        "load.errors",
        "load.http_errors",
        "load.latency_ms.p50",
        "load.latency_ms.p95",
        "load.latency_ms.p99",
        "load.latency_percentile_method",
        "load.latency_samples",
        "load.method",
        "load.requests",
        "load.requests_per_second",
        "load.transport_errors",
        "machine.arch",
        "machine.cpu_model",
        "machine.logical_cpus",
        "machine.os",
        "measured_at",
        "notes",
        "schema_version",
        "target.environment",
        "target.executable",
        "target.mode",
        "target.pid",
        "target.process_lookup",
        "target.url",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    for metric in [
        "artifacts.binary_size_bytes",
        "artifacts.docker_image_size_bytes",
        "process.cpu_time_seconds",
        "process.idle_rss_bytes",
        "process.peak_rss_bytes",
    ] {
        for field in ["method", "reason", "status", "unit", "value"] {
            expected.push(format!("{metric}.{field}"));
        }
    }
    expected.sort();
    assert_eq!(shapes[0], expected);
    let value = serde_json::to_value(sample(&estimate, Some(100.0))).unwrap();
    assert_eq!(value["schema_version"], "rullst.cli-footprint.v1");
    assert_eq!(value["energy"]["status"], "estimate");
    assert_eq!(value["carbon"]["estimated_terms"][0], "E");
    assert_eq!(value["load"]["requests_per_second"], 200.0);
    assert_eq!(value["process"]["idle_rss_bytes"]["status"], "not_measured");
}

#[test]
fn plain_rendering_shows_methods_and_not_measured_reasons() {
    let missing = Energy::decide(
        Err("RAPL counters exist but are not readable".into()),
        None,
        None,
    );
    let text = render::render(&sample(&missing, None), crate::ui::style::Style::PLAIN);
    assert!(!text.contains('\u{1b}'), "plain output has no escape codes");
    for expected in [
        "Rullst footprint · v13.0.0-test",
        "Requests/s",
        "200.0 req/s",
        "Latency p95",
        "3.800 ms",
        "Errors",
        "1 transport, 2 HTTP status >= 400",
        "Peak RSS      18.0 MiB",
        "Idle RSS      NOT MEASURED",
        "Docker image  NOT MEASURED",
        "Energy        NOT MEASURED",
        "RAPL counters exist but are not readable",
        "SCI = ((E × I) + M) / R",
        "I (grid)      NOT PROVIDED  ",
        "GET / · 2 s · 4 connections",
        "M (embodied)  not included",
        "NOT COMPUTED",
        "missing: E, I",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }

    let estimate = Energy::decide(Err("denied".into()), Some(1.5), Some(15.0));
    let text = render::render(
        &sample(&estimate, Some(100.0)),
        crate::ui::style::Style::PLAIN,
    );
    assert!(text.contains("22.500 J"), "{text}");
    assert!(text.contains("estimate: E = process CPU time"), "{text}");
    assert!(text.contains("gCO2e/request"), "{text}");
    assert!(text.contains("estimate: E is estimated"), "{text}");
}

#[test]
fn numbers_and_sizes_are_readable() {
    assert_eq!(render::bytes(512), "512 B");
    assert_eq!(render::bytes(1536), "1.5 KiB");
    assert_eq!(render::bytes(8 * 1024 * 1024), "8.0 MiB");
    assert_eq!(render::number(0.0), "0");
    assert_eq!(render::number(1234.56), "1234.6");
    assert_eq!(render::number(1.5), "1.500");
    assert_eq!(render::number(0.000_412), "4.120e-4");
}

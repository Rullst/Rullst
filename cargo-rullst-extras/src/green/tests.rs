//! JSON key stability and terminal rendering of a fixed report.
use super::load::Outcome;
use super::report::*;
use super::*;
use std::time::Duration;

fn outcome() -> Outcome {
    Outcome {
        responses: 400,
        transport_errors: 1,
        http_errors: 2,
        elapsed: Duration::from_secs(2),
    }
}

fn sample(bench: bool, grid_intensity: Option<f64>) -> Report {
    let benchmark = if bench {
        Benchmark::run("/", 2.0, 4, &outcome(), Ok(1.5))
    } else {
        Benchmark::not_run()
    };
    let sci_estimate = sci::estimate(sci::Inputs {
        cpu_time_seconds: benchmark.cpu_time_seconds.value,
        cpu_watts: bench.then_some(15.0),
        grid_intensity_g_per_kwh: grid_intensity,
        embodied_g: None,
        requests: benchmark.requests,
    });
    Report {
        schema: "rullst.green.v1",
        schema_version: report::SCHEMA_VERSION,
        cli_version: "13.0.0-test",
        measured_at: "2026-10-08T12:00:00Z".to_string(),
        machine: Machine {
            os: "linux",
            arch: "x86_64",
            logical_cpus: Some(4),
            cpu_model: Some("Example CPU".to_string()),
        },
        target: Target {
            mode: "release_build",
            url: "http://127.0.0.1:41234/".to_string(),
            pid: Some(4242),
            process_lookup: "started by green".to_string(),
            executable: Some("app".to_string()),
            environment: Some("RULLST_ENV=production HOST=127.0.0.1"),
        },
        measured: Measured {
            binary_size_bytes: Metric::measured(8 * 1024 * 1024, "bytes", "file size"),
            image_size_bytes: Metric::not_measured("bytes", "no image given"),
            idle_rss_bytes: Metric::measured(16 * 1024 * 1024, "bytes", "VmRSS"),
            dependency_count: Metric::measured(312, "packages", "Cargo.lock"),
        },
        benchmark,
        sci_estimate,
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
    let mut shapes = Vec::new();
    for report in [sample(true, Some(100.0)), sample(false, None)] {
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
        "benchmark.concurrency",
        "benchmark.duration_seconds",
        "benchmark.elapsed_seconds",
        "benchmark.http_errors",
        "benchmark.method",
        "benchmark.path",
        "benchmark.requests",
        "benchmark.status",
        "benchmark.transport_errors",
        "cli_version",
        "machine.arch",
        "machine.cpu_model",
        "machine.logical_cpus",
        "machine.os",
        "measured_at",
        "notes",
        "schema",
        "schema_version",
        "sci_estimate.energy_kwh",
        "sci_estimate.energy_method",
        "sci_estimate.formula",
        "sci_estimate.functional_unit",
        "sci_estimate.inputs.cpu_time_seconds",
        "sci_estimate.inputs.cpu_watts",
        "sci_estimate.inputs.embodied_g",
        "sci_estimate.inputs.grid_intensity_g_per_kwh",
        "sci_estimate.inputs.requests",
        "sci_estimate.label",
        "sci_estimate.missing_inputs",
        "sci_estimate.sci_g_per_request",
        "sci_estimate.standard",
        "sci_estimate.status",
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
        "benchmark.cpu_ms_per_request",
        "benchmark.cpu_time_seconds",
        "measured.binary_size_bytes",
        "measured.dependency_count",
        "measured.idle_rss_bytes",
        "measured.image_size_bytes",
    ] {
        for field in ["method", "reason", "status", "unit", "value"] {
            expected.push(format!("{metric}.{field}"));
        }
    }
    expected.sort();
    assert_eq!(shapes[0], expected);

    let value = serde_json::to_value(sample(true, Some(100.0))).unwrap();
    assert_eq!(value["schema_version"], "rullst.green.v1");
    assert_eq!(value["schema"], "rullst.green.v1");
    assert_eq!(value["benchmark"]["status"], "run");
    assert_eq!(value["benchmark"]["cpu_ms_per_request"]["value"], 3.75);
    assert_eq!(value["sci_estimate"]["status"], "estimate");
    assert_eq!(value["sci_estimate"]["label"], "estimate");
    assert_eq!(value["sci_estimate"]["inputs"]["cpu_watts"], 15.0);
    assert_eq!(value["measured"]["dependency_count"]["value"], 312);

    let value = serde_json::to_value(sample(false, None)).unwrap();
    assert_eq!(value["benchmark"]["status"], "not_run");
    assert_eq!(value["sci_estimate"]["status"], "not_computed");
    assert_eq!(value["sci_estimate"]["label"], "estimate");
    assert_eq!(
        value["benchmark"]["cpu_time_seconds"]["reason"],
        "no benchmark ran; pass --bench"
    );
}

#[test]
fn benchmark_cpu_figures_are_measured_or_explained() {
    let failed = Benchmark::run("/", 1.0, 1, &outcome(), Err("not readable"));
    assert_eq!(failed.status, "run");
    assert_eq!(
        failed.cpu_time_seconds.reason.as_deref(),
        Some("not readable")
    );
    assert_eq!(
        failed.cpu_ms_per_request.reason.as_deref(),
        Some("not readable")
    );
    let idle = Outcome {
        responses: 0,
        ..outcome()
    };
    let empty = Benchmark::run("/", 1.0, 1, &idle, Ok(0.25));
    assert_eq!(empty.cpu_time_seconds.value, Some(0.25));
    assert_eq!(
        empty.cpu_ms_per_request.reason.as_deref(),
        Some("no request completed")
    );
}

#[test]
fn plain_rendering_labels_measurements_and_the_estimate() {
    let plain = Style { color: false };
    let text = render::render(&sample(false, None), plain);
    assert!(!text.contains('\u{1b}'), "plain output has no escape codes");
    for expected in [
        "Rullst green · v13.0.0-test",
        "Binary size   8.0 MiB",
        "Image size    NOT MEASURED",
        "Idle RSS      16.0 MiB",
        "Dependencies  312",
        "Benchmark     NOT RUN",
        "pass --bench to measure CPU time per request",
        "SCI estimate (not a measurement)",
        "SCI           NOT ESTIMATED",
        "missing: cpu_time_seconds, cpu_watts, grid_intensity_g_per_kwh, requests",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }
    assert!(!text.contains("gCO2e/request"), "{text}");

    let text = render::render(&sample(true, Some(100.0)), plain);
    for expected in [
        "GET / · 2 s · 4 connections",
        "Requests      400",
        "1 transport errors, 2 HTTP status >= 400",
        "CPU time      1.500 s",
        "CPU/request   3.750 ms",
        "SCI = ((E × I) + M) / R",
        "estimate: CPU time 1.500 s × 15.000 W",
        "I (grid)      100.0 gCO2e/kWh",
        "M (embodied)  not included",
        "gCO2e/request",
        "estimate from the inputs above, not a measurement",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }
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

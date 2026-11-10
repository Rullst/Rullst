//! JSON key stability and terminal rendering of a fixed report.
use super::load::Outcome;
use super::report::*;
use super::*;
use std::time::Duration;

mod failures;
mod page_weight;
mod run;
pub(super) mod support;

fn outcome() -> Outcome {
    Outcome {
        responses: 400,
        transport_errors: 1,
        http_errors: 2,
        elapsed: Duration::from_secs(2),
        body_bytes: 600_000,
        encodings: ["br".to_string()].into(),
    }
}

fn sample(bench: bool, grid_intensity: Option<f64>) -> Report {
    let benchmark = if bench {
        Benchmark::run("/", 2.0, 4, &outcome(), Ok(1.5))
    } else {
        Benchmark::not_run("no benchmark ran; pass --bench")
    };
    let sci_estimate = sci::estimate(sci::Inputs {
        cpu_time_seconds: benchmark.cpu_time_seconds.value,
        cpu_watts: bench.then_some(15.0),
        grid_intensity_g_per_kwh: grid_intensity,
        embodied_g: None,
        requests: benchmark.requests,
        network_kwh_per_gb: None,
        response_bytes: benchmark.response_bytes.value,
    });
    Report {
        schema: "rullst.footprint.v1",
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
            url: Some("http://127.0.0.1:41234/".to_string()),
            pid: Some(4242),
            process_lookup: "started by footprint".to_string(),
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
        page_weight: if bench {
            page_weight::sample()
        } else {
            PageWeight::unmeasured("not_run", "no page was weighed; pass --page-weight")
        },
        sci_estimate,
        notes: NOTES,
        failure: None,
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
        "benchmark.accept_encoding",
        "benchmark.content_encodings",
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
        "failure",
        "machine.arch",
        "machine.cpu_model",
        "machine.logical_cpus",
        "machine.os",
        "measured_at",
        "notes",
        "page_weight.accept_encoding",
        "page_weight.first_visit.bytes",
        "page_weight.first_visit.from_cache",
        "page_weight.first_visit.not_modified",
        "page_weight.first_visit.requests",
        "page_weight.method",
        "page_weight.path",
        "page_weight.reason",
        "page_weight.repeat_visit.bytes",
        "page_weight.repeat_visit.from_cache",
        "page_weight.repeat_visit.not_modified",
        "page_weight.repeat_visit.requests",
        "page_weight.resources",
        "page_weight.status",
        "page_weight.third_party_origins",
        "page_weight.third_party_references",
        "page_weight.unfetched",
        "schema",
        "schema_version",
        "sci_estimate.cpu_energy_kwh",
        "sci_estimate.energy_kwh",
        "sci_estimate.energy_method",
        "sci_estimate.formula",
        "sci_estimate.functional_unit",
        "sci_estimate.inputs.cpu_time_seconds",
        "sci_estimate.inputs.cpu_watts",
        "sci_estimate.inputs.embodied_g",
        "sci_estimate.inputs.grid_intensity_g_per_kwh",
        "sci_estimate.inputs.network_kwh_per_gb",
        "sci_estimate.inputs.requests",
        "sci_estimate.inputs.response_bytes",
        "sci_estimate.label",
        "sci_estimate.missing_inputs",
        "sci_estimate.network_energy_kwh",
        "sci_estimate.network_method",
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
        "benchmark.bytes_per_request",
        "benchmark.cpu_ms_per_request",
        "benchmark.cpu_time_seconds",
        "benchmark.response_bytes",
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
    assert_eq!(value["schema_version"], "rullst.footprint.v1");
    assert_eq!(value["schema"], "rullst.footprint.v1");
    assert_eq!(value["benchmark"]["status"], "run");
    assert_eq!(value["benchmark"]["cpu_ms_per_request"]["value"], 3.75);
    assert_eq!(value["sci_estimate"]["status"], "estimate");
    assert_eq!(value["sci_estimate"]["label"], "estimate");
    assert_eq!(value["sci_estimate"]["inputs"]["cpu_watts"], 15.0);
    assert_eq!(value["measured"]["dependency_count"]["value"], 312);
    assert_eq!(value["benchmark"]["response_bytes"]["value"], 600_000);
    assert_eq!(value["benchmark"]["bytes_per_request"]["value"], 1500.0);
    assert_eq!(value["benchmark"]["accept_encoding"], "zstd, br, gzip");
    assert_eq!(
        value["benchmark"]["content_encodings"],
        serde_json::json!(["br"])
    );
    assert_eq!(value["page_weight"]["status"], "measured");
    assert_eq!(value["notes"].as_array().map(Vec::len), Some(NOTES.len()));

    let value = serde_json::to_value(sample(false, None)).unwrap();
    assert_eq!(value["benchmark"]["status"], "not_run");
    assert_eq!(value["sci_estimate"]["status"], "not_computed");
    assert_eq!(value["sci_estimate"]["label"], "estimate");
    assert_eq!(
        value["benchmark"]["cpu_time_seconds"]["reason"],
        "no benchmark ran; pass --bench"
    );
    assert_eq!(value["page_weight"]["status"], "not_run");
    assert_eq!(value["page_weight"]["resources"], serde_json::json!([]));
}

#[test]
fn the_command_is_footprint_and_green_is_gone() {
    let command = crate::command();
    assert!(command.find_subcommand("footprint").is_some());
    assert!(command.find_subcommand("green").is_none());
    assert!(command.try_get_matches_from(["rullst", "green"]).is_err());
    assert_eq!(report::SCHEMA_VERSION, "rullst.footprint.v1");
    assert_eq!(DOCS, "https://rullst.github.io/Rullst/book/footprint.html");
    let notes = NOTES.join(" ");
    assert!(
        notes.contains("not a certification, an environmental claim"),
        "{notes}"
    );
    assert!(
        notes.contains("responsible for how you word them"),
        "{notes}"
    );
}

#[test]
fn the_cpu_time_is_the_difference_of_two_samples() {
    assert_eq!(cpu_time(Some(1.5), Some(4.0)), Some(2.5));
    assert_eq!(cpu_time(Some(4.0), Some(4.0)), Some(0.0));
    assert_eq!(cpu_time(Some(4.0), Some(1.5)), None);
    assert_eq!(cpu_time(None, Some(1.5)), None);
    assert_eq!(cpu_time(Some(1.5), None), None);
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
    assert_eq!(
        empty.bytes_per_request.reason.as_deref(),
        Some("no request completed")
    );
    assert_eq!(empty.response_bytes.value, Some(600_000));
    let ran = Benchmark::run("/", 1.0, 1, &outcome(), Ok(0.25));
    assert_eq!(ran.bytes_per_request.value, Some(1500.0));
    assert_eq!(
        ran.cpu_time_seconds.method.as_deref(),
        Some(process::METHODS.cpu)
    );
    assert_eq!(ran.content_encodings, ["br"]);
}

#[test]
fn plain_rendering_labels_measurements_and_the_estimate() {
    let plain = Style { color: false };
    let text = render::render(&sample(false, None), plain);
    assert!(!text.contains('\u{1b}'), "plain output has no escape codes");
    for expected in [
        "Rullst footprint · v13.0.0-test",
        "Binary size   8.0 MiB",
        "Image size    NOT MEASURED",
        "Idle RSS      16.0 MiB",
        "Dependencies  312",
        "Benchmark     NOT RUN",
        "pass --bench to measure CPU time per request",
        "Page weight   NOT RUN",
        "no page was weighed; pass --page-weight",
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
        "Bytes/request 1.5 KiB",
        "585.9 KiB in all, as served (br)",
        "E (network)   not included",
        "pass --network-kwh-per-gb to include it",
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
    assert_eq!(render::bytes(1024), "1.0 KiB");
    assert_eq!(render::bytes(1024 * 1024), "1.0 MiB");
}

#[test]
fn the_target_rows_say_how_the_app_was_started_and_found() {
    let plain = Style { color: false };
    let text = render::render(&sample(false, None), plain);
    for expected in [
        "Mode          release build `app` started by footprint",
        "RULLST_ENV=production HOST=127.0.0.1",
        "URL           http://127.0.0.1:41234/",
        "Process       PID 4242",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }

    let mut unnamed = sample(false, None);
    unnamed.target.executable = None;
    let text = render::render(&unnamed, plain);
    assert!(
        text.contains("Mode          release build started by footprint"),
        "{text}"
    );

    let mut existing = sample(false, None);
    existing.target.mode = "existing_url";
    existing.target.pid = None;
    existing.target.environment = None;
    existing.target.process_lookup = "no listening socket found".to_string();
    let text = render::render(&existing, plain);
    assert!(
        text.contains("Mode          existing app (--url)"),
        "{text}"
    );
    assert!(text.contains("Process       NOT FOUND"), "{text}");
    assert!(text.contains("no listening socket found"), "{text}");
}

#[test]
fn start_up_help_and_the_recorded_environment_follow_the_requested_env() {
    use super::failure::start_fix;
    assert!(
        start_fix("production", "the app exited during start-up").starts_with(
            "Production needs the app's production settings: set the ones the error above names"
        )
    );
    assert!(
        start_fix("staging", "the app exited during start-up")
            .starts_with("Run the release binary with RULLST_ENV=staging")
    );
    assert_eq!(
        started_environment("staging"),
        "RULLST_ENV=staging HOST=127.0.0.1"
    );
    assert_eq!(
        started_environment("production"),
        "RULLST_ENV=production HOST=127.0.0.1"
    );
}

#[test]
fn unsampled_processes_are_explained() {
    assert_eq!(process_reason(None), "the app process was not identified");
    assert_eq!(process_reason(Some(4242)), process::METHODS.unreadable);
    let expected = if cfg!(target_os = "linux") {
        "/proc/<pid> was not readable"
    } else if cfg!(target_os = "macos") {
        "proc_pidinfo could not read the process"
    } else if cfg!(windows) {
        "the process could not be opened (OpenProcess)"
    } else {
        "process sampling supports Linux, macOS and Windows only"
    };
    assert_eq!(process::METHODS.unreadable, expected);
}

#[test]
fn the_network_term_is_rendered_with_its_inputs() {
    let mut report = sample(true, Some(100.0));
    report.sci_estimate = sci::estimate(sci::Inputs {
        network_kwh_per_gb: Some(0.059),
        ..report.sci_estimate.inputs.clone()
    });
    let text = render::render(&report, Style { color: false });
    assert!(text.contains("E (network)   3.540e-5 kWh"), "{text}");
    assert!(
        text.contains("estimate: 585.9 KiB served × 0.059 kWh/GB, included in E"),
        "{text}"
    );
    let mut silent = sample(true, Some(100.0));
    silent.benchmark.response_bytes = Metric::not_measured("bytes", "x");
    silent.benchmark.bytes_per_request = Metric::not_measured("bytes", "no bytes");
    let text = render::render(&silent, Style { color: false });
    assert!(text.contains("Bytes/request NOT MEASURED"), "{text}");
    assert!(text.contains("no bytes"), "{text}");
}

#[test]
fn page_weight_rows_show_both_visits_and_the_third_parties() {
    let plain = Style { color: false };
    let text = render::render(&sample(true, None), plain);
    for expected in [
        "First visit   46.9 KiB",
        "6 requests, GET / and its resources, as served (compressed: br)",
        "Repeat visit  1.2 KiB",
        "2 requests; 4 from the cache, 1 not modified (304)",
        "Third party   1 origins",
        "https://cdn.example (listed, not fetched; 1 resources)",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }
    assert!(!text.contains("Not fetched"), "{text}");

    let mut report = sample(true, None);
    report.page_weight.third_party_origins.clear();
    report.page_weight.resources[0].content_encoding = None;
    report.page_weight.unfetched = 3;
    let text = render::render(&report, plain);
    for expected in [
        "as served (uncompressed)",
        "Third party   none",
        "no other origin referenced",
        "Not fetched   3",
        "same-origin references beyond the first 200 responses",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }
    report.page_weight.unfetched = 1;
    assert!(render::render(&report, plain).contains("Not fetched   1"));

    let mut failed = sample(true, None);
    failed.page_weight = PageWeight::unmeasured("not_measured", "GET / failed: refused");
    let text = render::render(&failed, plain);
    assert!(text.contains("Page weight   NOT MEASURED"), "{text}");
    assert!(text.contains("GET / failed: refused"), "{text}");
}

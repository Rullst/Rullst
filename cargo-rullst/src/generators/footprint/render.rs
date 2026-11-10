//! Terminal tables for the footprint report: label, value and method on each
//! row; `NOT MEASURED` values keep their reason, and the SCI section says it
//! is an estimate. Plain under `NO_COLOR`, `CI` or when stdout is not a
//! terminal.
use super::report::{Metric, Report};
use crate::term::Style;

struct Row {
    label: &'static str,
    value: String,
    method: String,
    missing: bool,
}

fn row(label: &'static str, value: impl Into<String>, method: impl Into<String>) -> Row {
    Row {
        label,
        value: value.into(),
        method: method.into(),
        missing: false,
    }
}

fn missing(label: &'static str, value: &str, reason: impl Into<String>) -> Row {
    Row {
        label,
        value: value.to_string(),
        method: reason.into(),
        missing: true,
    }
}

pub(super) fn bytes(value: u64) -> String {
    const UNITS: [&str; 4] = ["KiB", "MiB", "GiB", "TiB"];
    if value < 1024 {
        return format!("{value} B");
    }
    let mut amount = value as f64;
    let mut unit = "B";
    for next in UNITS {
        if amount < 1024.0 {
            break;
        }
        amount /= 1024.0;
        unit = next;
    }
    format!("{amount:.1} {unit}")
}

/// Small quantities keep significant digits: `0.000412` rather than `0.000`.
pub(super) fn number(value: f64) -> String {
    let magnitude = value.abs();
    if magnitude == 0.0 {
        "0".to_string()
    } else if magnitude >= 100.0 {
        format!("{value:.1}")
    } else if magnitude >= 0.01 {
        format!("{value:.3}")
    } else {
        format!("{value:.3e}")
    }
}

/// Whole seconds without decimals (`5`), otherwise three decimals.
fn seconds(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        number(value)
    }
}

fn metric<T: Copy>(label: &'static str, metric: &Metric<T>, show: impl Fn(T) -> String) -> Row {
    match metric.value {
        Some(value) => row(
            label,
            show(value),
            metric.method.clone().unwrap_or_default(),
        ),
        None => missing(
            label,
            "NOT MEASURED",
            metric.reason.clone().unwrap_or_default(),
        ),
    }
}

fn target_rows(report: &Report) -> Vec<Row> {
    let target = &report.target;
    let mode = match (target.mode, &target.executable) {
        ("release_build", Some(name)) => format!("release build `{name}` started by footprint"),
        ("release_build", None) => "release build started by footprint".to_string(),
        _ => "existing app (--url)".to_string(),
    };
    let absent = if report.failure.is_some() {
        "NOT RUNNING"
    } else {
        "NOT FOUND"
    };
    let process = match target.pid {
        Some(pid) => row(
            "Process",
            format!("PID {pid}"),
            target.process_lookup.clone(),
        ),
        None => missing("Process", absent, target.process_lookup.clone()),
    };
    let url = match &target.url {
        Some(url) => row("URL", url.clone(), ""),
        None => missing("URL", "NONE", target.process_lookup.clone()),
    };
    vec![
        row("Mode", mode, target.environment.unwrap_or_default()),
        url,
        process,
    ]
}

fn measured_rows(report: &Report) -> Vec<Row> {
    let measured = &report.measured;
    vec![
        metric("Binary size", &measured.binary_size_bytes, bytes),
        metric("Image size", &measured.image_size_bytes, bytes),
        metric("Idle RSS", &measured.idle_rss_bytes, bytes),
        metric("Dependencies", &measured.dependency_count, |count| {
            count.to_string()
        }),
    ]
}

fn benchmark_rows(report: &Report) -> Vec<Row> {
    let benchmark = &report.benchmark;
    let (Some(path), Some(duration), Some(concurrency), Some(requests)) = (
        &benchmark.path,
        benchmark.duration_seconds,
        benchmark.concurrency,
        benchmark.requests,
    ) else {
        let reason = report.failure.as_ref().map_or(
            "pass --bench to measure CPU time per request",
            super::failure::Failure::unmeasured,
        );
        return vec![missing("Benchmark", "NOT RUN", reason)];
    };
    vec![
        row(
            "Scenario",
            format!(
                "GET {path} · {} s · {concurrency} connections",
                seconds(duration)
            ),
            "closed loop",
        ),
        row(
            "Requests",
            requests.to_string(),
            format!(
                "{} transport errors, {} HTTP status >= 400",
                benchmark.transport_errors.unwrap_or(0),
                benchmark.http_errors.unwrap_or(0)
            ),
        ),
        metric("CPU time", &benchmark.cpu_time_seconds, |value| {
            format!("{} s", number(value))
        }),
        metric("CPU/request", &benchmark.cpu_ms_per_request, |value| {
            format!("{} ms", number(value))
        }),
        match (
            benchmark.response_bytes.value,
            benchmark.bytes_per_request.value,
        ) {
            (Some(total), Some(each)) => row(
                "Bytes/request",
                bytes(each.round() as u64),
                format!(
                    "{} in all, as served ({})",
                    bytes(total),
                    benchmark.content_encodings.join(", ")
                ),
            ),
            _ => missing(
                "Bytes/request",
                "NOT MEASURED",
                benchmark
                    .bytes_per_request
                    .reason
                    .clone()
                    .unwrap_or_default(),
            ),
        },
    ]
}

fn page_rows(report: &Report) -> Vec<Row> {
    let page = &report.page_weight;
    let (Some(first), Some(first_requests), Some(repeat), Some(repeat_requests)) = (
        page.first_visit.bytes,
        page.first_visit.requests,
        page.repeat_visit.bytes,
        page.repeat_visit.requests,
    ) else {
        let value = if page.status == "not_run" {
            "NOT RUN"
        } else {
            "NOT MEASURED"
        };
        return vec![missing(
            "Page weight",
            value,
            page.reason.clone().unwrap_or_default(),
        )];
    };
    let mut encodings: Vec<&str> = page
        .resources
        .iter()
        .filter_map(|resource| resource.content_encoding.as_deref())
        .collect();
    encodings.sort_unstable();
    encodings.dedup();
    let served = if encodings.is_empty() {
        "uncompressed".to_string()
    } else {
        format!("compressed: {}", encodings.join(", "))
    };
    let mut rows = vec![
        row(
            "First visit",
            bytes(first),
            format!(
                "{first_requests} requests, GET {} and its resources, as served ({served})",
                page.path.as_deref().unwrap_or("/")
            ),
        ),
        row(
            "Repeat visit",
            bytes(repeat),
            format!(
                "{repeat_requests} requests; {} from the cache, {} not modified (304)",
                page.repeat_visit.from_cache.unwrap_or(0),
                page.repeat_visit.not_modified.unwrap_or(0)
            ),
        ),
    ];
    rows.push(if page.third_party_origins.is_empty() {
        row("Third party", "none", "no other origin referenced")
    } else {
        row(
            "Third party",
            format!("{} origins", page.third_party_origins.len()),
            format!(
                "{} (listed, not fetched; {} resources)",
                page.third_party_origins.join(", "),
                page.third_party_references
            ),
        )
    });
    if page.unfetched > 0 {
        rows.push(missing(
            "Not fetched",
            &page.unfetched.to_string(),
            format!(
                "same-origin references beyond the first {} responses",
                super::page::MAX_RESOURCES
            ),
        ));
    }
    rows
}

fn estimate_rows(report: &Report) -> Vec<Row> {
    let sci = &report.sci_estimate;
    let inputs = &sci.inputs;
    let Some(value) = sci.sci_g_per_request else {
        return vec![missing(
            "SCI",
            "NOT ESTIMATED",
            format!(
                "needs --bench, --cpu-watts and --grid-intensity; missing: {}",
                sci.missing_inputs.join(", ")
            ),
        )];
    };
    let energy = format!(
        "estimate: CPU time {} s × {} W",
        number(inputs.cpu_time_seconds.unwrap_or_default()),
        number(inputs.cpu_watts.unwrap_or_default())
    );
    let network = match (sci.network_energy_kwh, inputs.network_kwh_per_gb) {
        (Some(energy), Some(per_gb)) => row(
            "E (network)",
            format!("{} kWh", number(energy)),
            format!(
                "estimate: {} served × {} kWh/GB, included in E",
                bytes(inputs.response_bytes.unwrap_or_default()),
                number(per_gb)
            ),
        ),
        _ => row(
            "E (network)",
            "not included",
            "pass --network-kwh-per-gb to include it",
        ),
    };
    let embodied = match inputs.embodied_g {
        Some(grams) => row(
            "M (embodied)",
            format!("{} gCO2e", number(grams)),
            "supplied with --embodied",
        ),
        None => row(
            "M (embodied)",
            "not included",
            "pass --embodied to include it",
        ),
    };
    vec![
        row("Formula", sci.formula, sci.standard),
        row(
            "E (energy)",
            format!("{} kWh", number(sci.energy_kwh.unwrap_or_default())),
            energy,
        ),
        network,
        row(
            "I (grid)",
            format!(
                "{} gCO2e/kWh",
                number(inputs.grid_intensity_g_per_kwh.unwrap_or_default())
            ),
            "supplied with --grid-intensity",
        ),
        embodied,
        row(
            "R (requests)",
            inputs.requests.unwrap_or_default().to_string(),
            sci.functional_unit,
        ),
        row(
            "SCI",
            format!("{} gCO2e/request", number(value)),
            "estimate from the inputs above, not a measurement",
        ),
    ]
}

pub(super) fn render(report: &Report, style: Style) -> String {
    let machine = &report.machine;
    let mut out = format!(
        "{} {}\n",
        style.bold("Rullst footprint"),
        style.dim(&format!("· v{}", report.cli_version))
    );
    let cpus = machine
        .logical_cpus
        .map_or_else(String::new, |count| format!(" · {count} logical CPUs"));
    let model = machine
        .cpu_model
        .as_deref()
        .map_or_else(String::new, |model| format!(" · {model}"));
    out.push_str(&style.dim(&format!(
        "Measured {} on {} {}{cpus}{model}\n\n",
        report.measured_at, machine.os, machine.arch
    )));
    let sections = [
        ("Target", target_rows(report)),
        ("Measured", measured_rows(report)),
        ("Benchmark", benchmark_rows(report)),
        ("Page weight", page_rows(report)),
        ("SCI estimate (not a measurement)", estimate_rows(report)),
    ];
    let value_width = sections
        .iter()
        .flat_map(|(_, rows)| rows)
        .map(|row| row.value.chars().count())
        .max()
        .unwrap_or(0);
    for (title, rows) in &sections {
        out.push_str(&format!("{}\n", style.bold(title)));
        for row in rows {
            let label = format!("{:<14}", row.label);
            let value = format!("{:<value_width$}", row.value);
            let value = if row.missing {
                style.yellow(&value)
            } else {
                value
            };
            out.push_str(
                format!(
                    "  {}{}  {}",
                    style.dim(&label),
                    value,
                    style.dim(&row.method)
                )
                .trim_end(),
            );
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str(&super::production::render(
        &report.production,
        report.benchmark.bytes_per_request.value,
        style,
    ));
    out.push_str(&super::budget::render(&report.budgets, style));
    if let Some(failure) = &report.failure {
        out.push_str(&format!("{}\n", style.bold("Not measured")));
        out.push_str(&format!(
            "  {}\n",
            style.yellow(&format!("{}: {}", failure.title, failure.unmeasured()))
        ));
        out.push_str(&format!(
            "  {}\n\n",
            style.dim("Figures that need the running app are NOT MEASURED; the error and its fix follow on stderr.")
        ));
    }
    out.push_str(&format!("{}\n", style.bold("Notes")));
    for note in report.notes {
        out.push_str(&format!("  - {}\n", style.dim(note)));
    }
    out
}

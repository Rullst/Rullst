//! Terminal tables for the green report: label, value and method on each
//! row; `NOT MEASURED` values keep their reason, and the SCI section says it
//! is an estimate. Plain under `NO_COLOR`, `CI` or when stdout is not a
//! terminal.
use super::report::{Metric, Report};
use cargo_rullst::term::Style;

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
        ("release_build", Some(name)) => format!("release build `{name}` started by green"),
        ("release_build", None) => "release build started by green".to_string(),
        _ => "existing app (--url)".to_string(),
    };
    let process = match target.pid {
        Some(pid) => row(
            "Process",
            format!("PID {pid}"),
            target.process_lookup.clone(),
        ),
        None => missing("Process", "NOT FOUND", target.process_lookup.clone()),
    };
    vec![
        row("Mode", mode, target.environment.unwrap_or_default()),
        row("URL", target.url.clone(), ""),
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
        return vec![missing(
            "Benchmark",
            "NOT RUN",
            "pass --bench to measure CPU time per request",
        )];
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
    ]
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
        style.bold("Rullst green"),
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
    out.push_str(&format!("{}\n", style.bold("Notes")));
    for note in report.notes {
        out.push_str(&format!("  - {}\n", style.dim(note)));
    }
    out
}

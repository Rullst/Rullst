//! Terminal tables for the footprint report: label, value and method on each
//! row; `NOT MEASURED` values keep their reason. Plain under `NO_COLOR` or
//! when stdout is not a terminal.
use super::report::{Metric, Report};
use crate::ui::style::{self, Style};

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

fn optional(value: Option<f64>, unit: &str) -> String {
    value.map_or_else(
        || "n/a".to_string(),
        |value| format!("{} {unit}", number(value)),
    )
}

fn sections(report: &Report) -> Vec<(&'static str, Vec<Row>)> {
    let target = &report.target;
    let load = &report.load;
    let inputs = &report.inputs;
    let mode = match (target.mode, &target.executable) {
        ("release_build", Some(name)) => format!("release build `{name}` started by footprint"),
        ("release_build", None) => "release build started by footprint".to_string(),
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
    let target_rows = vec![
        row("Mode", mode, target.environment.unwrap_or_default()),
        row("URL", target.url.clone(), ""),
        process,
        row(
            "Scenario",
            format!(
                "GET {} · {} s · {} connections",
                inputs.path,
                seconds(inputs.duration_seconds),
                inputs.concurrency
            ),
            "closed loop",
        ),
    ];
    let latency = |label, value: Option<f64>| match value {
        Some(value) => row(
            label,
            format!("{} ms", number(value)),
            format!(
                "{}, {} samples",
                load.latency_percentile_method, load.latency_samples
            ),
        ),
        None => missing(label, "n/a", "no request completed"),
    };
    let load_rows = vec![
        row(
            "Requests/s",
            optional(load.requests_per_second, "req/s"),
            format!(
                "{} responses / {} s wall time",
                load.requests,
                number(load.elapsed_seconds)
            ),
        ),
        latency("Latency p50", load.latency_ms.p50),
        latency("Latency p95", load.latency_ms.p95),
        latency("Latency p99", load.latency_ms.p99),
        row(
            "Errors",
            load.errors.to_string(),
            format!(
                "{} transport, {} HTTP status >= 400",
                load.transport_errors, load.http_errors
            ),
        ),
    ];
    let process_rows = vec![
        metric("CPU time", &report.process.cpu_time_seconds, |value| {
            format!("{} s", number(value))
        }),
        metric("Peak RSS", &report.process.peak_rss_bytes, bytes),
        metric("Idle RSS", &report.process.idle_rss_bytes, bytes),
    ];
    let artifact_rows = vec![
        metric("Binary size", &report.artifacts.binary_size_bytes, bytes),
        metric(
            "Docker image",
            &report.artifacts.docker_image_size_bytes,
            bytes,
        ),
    ];
    let energy = &report.energy;
    let energy_rows = match energy.joules {
        Some(joules) => vec![
            row(
                "Energy",
                format!("{} J", number(joules)),
                energy.method.clone().unwrap_or_default(),
            ),
            row(
                "Per request",
                optional(energy.joules_per_request, "J"),
                "energy / requests",
            ),
        ],
        None => vec![missing(
            "Energy",
            "NOT MEASURED",
            energy.reason.clone().unwrap_or_default(),
        )],
    };
    let carbon = &report.carbon;
    let e = match carbon.e_kwh {
        Some(value) => row(
            "E (energy)",
            format!("{} kWh", number(value)),
            carbon.e_source.clone(),
        ),
        None => missing("E (energy)", "NOT MEASURED", "see Energy"),
    };
    let i = match carbon.i_g_per_kwh {
        Some(value) => row(
            "I (grid)",
            format!("{} gCO2e/kWh", number(value)),
            carbon.i_source,
        ),
        None => missing(
            "I (grid)",
            "NOT PROVIDED",
            "pass --grid-intensity from your own source",
        ),
    };
    let m = match carbon.m_g {
        Some(value) => row(
            "M (embodied)",
            format!("{} gCO2e", number(value)),
            carbon.m_source,
        ),
        None => row(
            "M (embodied)",
            "not included",
            "pass --embodied to include it",
        ),
    };
    let sci = match carbon.sci_g_per_request {
        Some(value) => {
            let estimates = if carbon.estimated_terms.is_empty() {
                "no estimated terms".to_string()
            } else {
                format!(
                    "estimate: {} is estimated",
                    carbon.estimated_terms.join(", ")
                )
            };
            row("SCI", format!("{} gCO2e/request", number(value)), estimates)
        }
        None => missing(
            "SCI",
            "NOT COMPUTED",
            format!("missing: {}", carbon.missing_terms.join(", ")),
        ),
    };
    let carbon_rows = vec![
        row("Formula", carbon.formula, carbon.standard),
        e,
        i,
        m,
        row(
            "R (requests)",
            carbon.r_requests.to_string(),
            carbon.functional_unit,
        ),
        sci,
    ];
    vec![
        ("Target", target_rows),
        ("Load", load_rows),
        ("Process", process_rows),
        ("Artifacts", artifact_rows),
        ("Energy", energy_rows),
        ("Carbon (SCI)", carbon_rows),
    ]
}

pub(super) fn render(report: &Report, style: Style) -> String {
    let machine = &report.machine;
    let mut out = format!(
        "{} {}\n",
        style.bold("Rullst footprint", style::BRIGHT),
        style.paint(&format!("· v{}", report.cli_version), style::MUTED)
    );
    let cpus = machine
        .logical_cpus
        .map_or_else(String::new, |count| format!(" · {count} logical CPUs"));
    let model = machine
        .cpu_model
        .as_deref()
        .map_or_else(String::new, |model| format!(" · {model}"));
    out.push_str(&style.paint(
        &format!(
            "Measured {} on {} {}{cpus}{model}\n\n",
            report.measured_at, machine.os, machine.arch
        ),
        style::MUTED,
    ));
    let sections = sections(report);
    let value_width = sections
        .iter()
        .flat_map(|(_, rows)| rows)
        .map(|row| row.value.chars().count())
        .max()
        .unwrap_or(0);
    for (title, rows) in &sections {
        out.push_str(&format!("{}\n", style.bold(title, style::BRIGHT)));
        for row in rows {
            let label = format!("{:<14}", row.label);
            let value = format!("{:<value_width$}", row.value);
            let color = if row.missing {
                style::WARN
            } else {
                style::BRIGHT
            };
            out.push_str(
                format!(
                    "  {}{}  {}",
                    style.paint(&label, style::MUTED),
                    style.paint(&value, color),
                    style.paint(&row.method, style::MUTED)
                )
                .trim_end(),
            );
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str(&format!("{}\n", style.bold("Notes", style::BRIGHT)));
    for note in report.notes {
        out.push_str(&format!("  - {}\n", style.paint(note, style::MUTED)));
    }
    out
}

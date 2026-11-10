//! `cargo rullst footprint`: measured resource figures for a Rullst app, and
//! an SCI carbon estimate only from inputs the user supplies. The report is
//! a measurement and an estimate, never a certification or an environmental
//! claim.
//!
//! Measured: the release binary size, a container image's size when one is
//! named (`--image`), the idle resident memory of the app, the dependency
//! count from `Cargo.lock`; with `--bench`, the app's CPU time and the bytes
//! it served during a bounded loopback load, per request; with
//! `--page-weight`, the bytes and requests of a page and its same-origin
//! resources on a first and a repeat visit ([`page`]). Estimated, and
//! labelled so: `SCI = ((E × I) + M) / R`, where E is the measured CPU time ×
//! the `--cpu-watts` figure, plus the served bytes × `--network-kwh-per-gb`
//! when given, and I (`--grid-intensity`) and M (`--embodied`) are the
//! user's. Footprint measures no energy and fetches no carbon data.
//!
//! Without `--url` it builds the project's release binary (reusing an
//! existing build), starts it in production mode (or `--env staging`) on a
//! free loopback port, measures it and stops it. With `--url` it measures an
//! app already running on loopback. The process exits 0 whenever the
//! measurement ran. When the build fails or the app does not start or
//! answer, the report still carries the figures that need no running app,
//! with the failure inside, and the process exits 1.

mod artifacts;
mod failure;
mod load;
mod options;
mod page;
mod process;
#[cfg(target_os = "linux")]
mod procfs;
mod render;
mod report;
mod sci;
mod subject;
mod target;
#[cfg(test)]
mod tests;

pub(crate) use options::command;

use crate::term::{Style, TermEnv};
use clap::ArgMatches;
use failure::Failure;
use options::Options;
use page::PageWeight;
use report::{Benchmark, Machine, Measured, Metric, Report};
use std::error::Error;
use std::io::Write;
#[cfg(test)]
use subject::{file_name, started_environment};

const DOCS: &str = "https://rullst.github.io/Rullst/book/footprint.html";

pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let options = Options::from_matches(matches);
    let workers = std::thread::available_parallelism()
        .map_or(2, std::num::NonZeroUsize::get)
        .clamp(1, 4);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .build()?;
    let report = runtime.block_on(measure(&options))?;
    let mut stdout = std::io::stdout().lock();
    if options.json {
        writeln!(stdout, "{}", serde_json::to_string_pretty(&report)?)?;
    } else {
        let style = Style {
            color: TermEnv::detect().color(),
        };
        write!(stdout, "{}", render::render(&report, style))?;
    }
    stdout.flush()?;
    match &report.failure {
        Some(failure) => Err(failure.report()),
        None => Ok(()),
    }
}

async fn measure(options: &Options) -> Result<Report, Box<dyn Error>> {
    let client = load::client()?;
    let mut subject = match &options.url {
        Some(base) => subject::existing(&client, base, &options.path).await?,
        None => subject::spawned(&client, &options.path, options.environment).await?,
    };
    let pid = subject.target.pid;
    let idle_rss = pid.and_then(process::rss_bytes);
    let unmeasured = subject
        .failure
        .as_ref()
        .map_or_else(|| process_reason(pid), Failure::unmeasured);
    let page_weight = match (options.page_weight, &subject.base) {
        (true, Some(base)) => {
            if !options.json {
                eprintln!(
                    "Weighing GET {} and the resources it references...",
                    options.path
                );
            }
            page::measure(&client, base, &options.path).await
        }
        (true, None) => PageWeight::unmeasured("not_measured", unmeasured),
        (false, _) => PageWeight::unmeasured("not_run", "no page was weighed; pass --page-weight"),
    };
    let benchmark = if let (true, Some(base)) = (options.bench, &subject.base) {
        let url = target::request_url(base, &options.path);
        if !options.json {
            eprintln!(
                "Benchmarking GET {} for {} s with {} connections...",
                options.path,
                options.duration.as_secs_f64(),
                options.concurrency
            );
        }
        let before = pid.and_then(process::cpu_seconds);
        let outcome = load::run(&client, &url, options.duration, options.concurrency).await;
        let after = pid.and_then(process::cpu_seconds);
        Benchmark::run(
            &options.path,
            options.duration.as_secs_f64(),
            options.concurrency,
            &outcome,
            cpu_time(before, after).ok_or_else(|| process_reason(pid)),
        )
    } else if options.bench {
        Benchmark::not_run(unmeasured)
    } else {
        Benchmark::not_run("no benchmark ran; pass --bench")
    };
    if let Some(app) = subject.owned.as_mut() {
        app.stop();
    }
    let measured = Measured {
        binary_size_bytes: match (&subject.failure, subject.executable.as_deref()) {
            (Some(failure), None) => Metric::not_measured("bytes", failure.unmeasured()),
            (_, executable) => artifacts::binary_size(executable, subject.executable_method),
        },
        image_size_bytes: artifacts::image_size(options.image.as_deref()),
        idle_rss_bytes: match idle_rss {
            Some(bytes) => Metric::measured(bytes, "bytes", process::METHODS.rss),
            None => Metric::not_measured("bytes", unmeasured),
        },
        dependency_count: artifacts::dependency_count(&std::env::current_dir()?),
    };
    let sci_estimate = sci::estimate(sci::Inputs {
        cpu_time_seconds: benchmark.cpu_time_seconds.value,
        cpu_watts: options.cpu_watts,
        grid_intensity_g_per_kwh: options.grid_intensity,
        embodied_g: options.embodied,
        requests: benchmark.requests,
        network_kwh_per_gb: options.network_kwh_per_gb,
        response_bytes: benchmark.response_bytes.value,
    });
    Ok(Report {
        schema_version: report::SCHEMA_VERSION,
        schema: report::SCHEMA_VERSION,
        cli_version: env!("CARGO_PKG_VERSION"),
        measured_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        machine: Machine::current(),
        target: subject.target.clone(),
        measured,
        benchmark,
        page_weight,
        sci_estimate,
        notes: report::NOTES,
        failure: subject.failure,
    })
}

/// The CPU seconds between two cumulative samples, unless one is missing or
/// time went backwards.
fn cpu_time(before: Option<f64>, after: Option<f64>) -> Option<f64> {
    match (before, after) {
        (Some(start), Some(end)) if end >= start => Some(end - start),
        _ => None,
    }
}

/// Why a process figure is missing: no process, or this platform could not
/// read it.
fn process_reason(pid: Option<u32>) -> &'static str {
    match pid {
        None => "the app process was not identified",
        Some(_) => process::METHODS.unreadable,
    }
}

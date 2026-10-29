//! `cargo rullst green`: measured efficiency figures for a Rullst app, and
//! an SCI carbon estimate only from inputs the user supplies.
//!
//! Measured: the release binary size, a container image's size when one is
//! named (`--image`), the idle resident memory of the started app, the
//! dependency count from `Cargo.lock` and, with `--bench`, the app's CPU time
//! during a bounded loopback load and per request. Estimated, and labelled
//! so: `SCI = ((E × I) + M) / R`, where E is the measured CPU time × the
//! `--cpu-watts` figure and I (`--grid-intensity`) and M (`--embodied`) are
//! the user's. Green measures no energy and fetches no carbon data.
//!
//! Without `--url` it builds the project's release binary (reusing an
//! existing build), starts it in production mode (or `--env staging`) on a
//! free loopback port, measures it and stops it. With `--url` it measures an
//! app already running on loopback. The process exits 0 whenever the
//! measurement ran.

mod artifacts;
mod load;
mod options;
mod procfs;
mod render;
mod report;
mod sci;
mod target;
#[cfg(test)]
mod tests;

pub(crate) use options::command;

use crate::term::{Style, TermEnv};
use crate::ui::error_report::{self, AlreadyReported, Friendly, ProjectRequired};
use clap::ArgMatches;
use options::Options;
use report::{Benchmark, Machine, Measured, Metric, Report, Target};
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};

const DOCS: &str = "https://rullst.github.io/Rullst/book/green.html";

/// Prints a friendly report on stderr; the exit status is 1.
fn failure(title: &str, happened: String, fix: &str) -> Box<dyn Error> {
    let friendly = Friendly {
        title: title.to_string(),
        happened: happened.clone(),
        fix: Some(fix.to_string()),
        docs: Some(DOCS),
    };
    eprint!(
        "{}",
        error_report::render(
            &friendly,
            &[],
            None,
            false,
            crate::ui::style::Style::stderr()
        )
    );
    AlreadyReported::new(1, happened).into()
}

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
    Ok(())
}

/// The app being measured: its URL, PID and executable, plus the process
/// started by green (stopped on drop).
struct Subject {
    base: reqwest::Url,
    target: Target,
    executable: Option<PathBuf>,
    executable_method: &'static str,
    owned: Option<target::OwnedApp>,
}

async fn existing(
    client: &reqwest::Client,
    base: &reqwest::Url,
    path: &str,
) -> Result<Subject, Box<dyn Error>> {
    let url = target::request_url(base, path);
    let probe = client
        .get(url.clone())
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;
    if probe.is_err() {
        return Err(failure(
            "App not reachable",
            format!("Nothing answered on {url}."),
            "Start the app first (for example `cargo rullst dev` or the release binary), or omit --url to let green build and start it.",
        ));
    }
    let port = base.port_or_known_default().unwrap_or(80);
    let (pid, lookup) = match target::listening_process(port) {
        target::Listener::Found(pid) => (
            Some(pid),
            format!(
                "listening on port {port} (/proc/net/tcp socket inode matched in /proc/<pid>/fd)"
            ),
        ),
        target::Listener::Unknown(reason) => (None, reason.to_string()),
    };
    let executable = pid.map(|pid| PathBuf::from(format!("/proc/{pid}/exe")));
    let name = executable
        .as_deref()
        .and_then(|link| std::fs::read_link(link).ok())
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        });
    Ok(Subject {
        base: base.clone(),
        target: Target {
            mode: "existing_url",
            url: url.to_string(),
            pid,
            process_lookup: lookup,
            executable: name,
            environment: None,
        },
        executable,
        executable_method: "file size of the running executable (/proc/<pid>/exe)",
        owned: None,
    })
}

async fn spawned(
    client: &reqwest::Client,
    path: &str,
    environment: &'static str,
) -> Result<Subject, Box<dyn Error>> {
    if crate::generators::platform_name::package_name(Path::new("Cargo.toml")).is_none() {
        return Err(ProjectRequired.into());
    }
    eprintln!(
        "Building the release binary (cargo build --release; an existing build is reused)..."
    );
    let binary = match crate::generators::dev::compile_release_in(Path::new(".")).await {
        Ok(binary) => binary,
        Err(error) => {
            return Err(failure(
                "Release build failed",
                error_report::sanitize(&error.to_string()),
                "Fix the build errors (`cargo build --release`), then run green again.",
            ));
        }
    };
    let port = target::free_port()?;
    let base = target::validate_url(&format!("http://127.0.0.1:{port}"))?;
    let url = target::request_url(&base, path);
    eprintln!("Starting the app with RULLST_ENV={environment} on {base} ...");
    let mut app = target::OwnedApp::spawn(&binary, port, environment).map_err(|error| {
        failure(
            "App could not be started",
            format!("Starting {} failed: {error}.", binary.display()),
            "Check that the release binary runs: `cargo run --release`.",
        )
    })?;
    if let Err(error) = app.wait_ready(client, &url).await {
        let tail = app.stderr_tail();
        let detail = if tail.is_empty() {
            String::new()
        } else {
            format!("\nLast stderr lines:\n{tail}")
        };
        let fix = if environment == "production" {
            "Production needs the app's production settings (the SaaS starter's real billing credentials, for example): set them, measure with --env staging, or start the app yourself and measure it with --url."
        } else {
            "Run the release binary with RULLST_ENV=staging to see why it does not serve requests, or measure a running app with --url."
        };
        return Err(failure(
            "App could not be started",
            format!("{error}.{detail}"),
            fix,
        ));
    }
    let name = binary
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    Ok(Subject {
        base,
        target: Target {
            mode: "release_build",
            url: url.to_string(),
            pid: Some(app.pid()),
            process_lookup: "started by green".to_string(),
            executable: name,
            environment: Some(if environment == "staging" {
                "RULLST_ENV=staging HOST=127.0.0.1"
            } else {
                "RULLST_ENV=production HOST=127.0.0.1"
            }),
        },
        executable: Some(binary),
        executable_method: "file size of the release binary built by cargo",
        owned: Some(app),
    })
}

async fn measure(options: &Options) -> Result<Report, Box<dyn Error>> {
    let client = load::client()?;
    let mut subject = match &options.url {
        Some(base) => existing(&client, base, &options.path).await?,
        None => spawned(&client, &options.path, options.environment).await?,
    };
    let pid = subject.target.pid;
    let idle_rss = pid.and_then(procfs::sample_rss);
    let benchmark = if options.bench {
        let url = target::request_url(&subject.base, &options.path);
        if !options.json {
            eprintln!(
                "Benchmarking GET {} for {} s with {} connections...",
                options.path,
                options.duration.as_secs_f64(),
                options.concurrency
            );
        }
        let before = pid.and_then(procfs::sample_cpu_ticks);
        let outcome = load::run(&client, &url, options.duration, options.concurrency).await;
        let after = pid.and_then(procfs::sample_cpu_ticks);
        let cpu_seconds = match (before, after, procfs::ticks_per_second()) {
            (Some(start), Some(end), Some(rate)) => procfs::cpu_seconds(start, end, rate),
            _ => None,
        }
        .ok_or_else(|| process_reason(pid));
        Benchmark::run(
            &options.path,
            options.duration.as_secs_f64(),
            options.concurrency,
            &outcome,
            cpu_seconds,
        )
    } else {
        Benchmark::not_run()
    };
    if let Some(app) = subject.owned.as_mut() {
        app.stop();
    }
    let measured = Measured {
        binary_size_bytes: artifacts::binary_size(
            subject.executable.as_deref(),
            subject.executable_method,
        ),
        image_size_bytes: artifacts::image_size(options.image.as_deref()),
        idle_rss_bytes: match idle_rss {
            Some(bytes) => Metric::measured(
                bytes,
                "bytes",
                "VmRSS after start-up, before any benchmark request (/proc/<pid>/status)",
            ),
            None => Metric::not_measured("bytes", process_reason(pid)),
        },
        dependency_count: artifacts::dependency_count(&std::env::current_dir()?),
    };
    let sci_estimate = sci::estimate(sci::Inputs {
        cpu_time_seconds: benchmark.cpu_time_seconds.value,
        cpu_watts: options.cpu_watts,
        grid_intensity_g_per_kwh: options.grid_intensity,
        embodied_g: options.embodied,
        requests: benchmark.requests,
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
        sci_estimate,
        notes: report::NOTES,
    })
}

fn process_reason(pid: Option<u32>) -> &'static str {
    if pid.is_none() {
        "the app process was not identified"
    } else if cfg!(target_os = "linux") {
        "/proc/<pid> was not readable"
    } else {
        "process sampling uses /proc and is Linux-only"
    }
}

//! `cargo rullst footprint`: a bounded local load run that reports what was
//! measured and how, what was estimated and from which inputs, and what was
//! not measured. It talks to loopback only and never fetches carbon data.
//!
//! Without `--url` it builds the project's release binary (reusing an
//! existing build), starts it in production mode on a free loopback port,
//! measures it and stops it. With `--url` it measures an app already running
//! on loopback. The process exits 0 whenever the measurement ran.

mod artifacts;
mod energy;
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

use crate::ui::error_report::{self, AlreadyReported, Friendly, ProjectRequired};
use crate::ui::style::Style;
use clap::ArgMatches;
use options::Options;
use report::{Artifacts, EnergyView, Inputs, Load, Machine, Metric, Process, Report, Target};
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};

const DOCS: &str = "https://rullst.github.io/Rullst/book/footprint.html";

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
        error_report::render(&friendly, &[], None, false, Style::stderr())
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
        write!(stdout, "{}", render::render(&report, Style::stdout()))?;
    }
    stdout.flush()?;
    Ok(())
}

/// Samples taken around the load window.
struct Window {
    ticks: Option<u64>,
    energy: Option<Vec<u64>>,
}

fn sample(pid: Option<u32>, zones: &[energy::Zone]) -> Window {
    Window {
        ticks: pid.and_then(procfs::sample_cpu_ticks),
        energy: if zones.is_empty() {
            None
        } else {
            energy::read(zones)
        },
    }
}

/// The app being measured: its URL, PID and executable, plus the process
/// started by footprint (stopped on drop).
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
            "Start the app first (for example `cargo rullst dev` or the release binary), or omit --url to let footprint build and start it.",
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

async fn spawned(client: &reqwest::Client, path: &str) -> Result<Subject, Box<dyn Error>> {
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
                "Fix the build errors (`cargo build --release`), then run footprint again.",
            ));
        }
    };
    let port = target::free_port()?;
    let base = target::validate_url(&format!("http://127.0.0.1:{port}"))?;
    let url = target::request_url(&base, path);
    eprintln!("Starting the app in production mode on {base} ...");
    let mut app = target::OwnedApp::spawn(&binary, port).map_err(|error| {
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
        return Err(failure(
            "App could not be started",
            format!("{error}.{detail}"),
            "Run the release binary with RULLST_ENV=production to see why it does not serve requests, or measure a running app with --url.",
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
            process_lookup: "started by footprint".to_string(),
            executable: name,
            environment: Some("RULLST_ENV=production HOST=127.0.0.1"),
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
        None => spawned(&client, &options.path).await?,
    };
    let pid = subject.target.pid;
    let zones = energy::discover(Path::new(energy::POWERCAP_ROOT));
    let readable = zones.as_deref().unwrap_or_default();
    let (_, idle_rss) = pid.map_or((None, None), procfs::sample_memory);
    let url = target::request_url(&subject.base, &options.path);
    if !options.json {
        eprintln!(
            "Measuring GET {} for {} s with {} connections...",
            options.path,
            options.duration.as_secs_f64(),
            options.concurrency
        );
    }
    let before = sample(pid, readable);
    let outcome = load::run(&client, &url, options.duration, options.concurrency).await;
    let after = sample(pid, readable);
    let (peak_rss, _) = pid.map_or((None, None), procfs::sample_memory);
    if let Some(app) = subject.owned.as_mut() {
        app.stop();
    }

    let cpu_seconds = match (before.ticks, after.ticks, procfs::ticks_per_second()) {
        (Some(start), Some(end), Some(rate)) => procfs::cpu_seconds(start, end, rate),
        _ => None,
    };
    let rapl = match (&zones, &before.energy, &after.energy) {
        (Ok(zones), Some(start), Some(end)) => energy::joules(zones, start, end)
            .map(|joules| (joules, zones.iter().map(|zone| zone.name.clone()).collect()))
            .ok_or_else(|| "RAPL counters could not be read twice".to_string()),
        (Err(reason), _, _) => Err(reason.clone()),
        _ => Err("RAPL counters could not be read twice".to_string()),
    };
    let energy = energy::Energy::decide(rapl, cpu_seconds, options.cpu_watts);
    let requests = outcome.responses;
    let proc_method = |file: &str, what: &str| format!("{what} (/proc/<pid>/{file})");
    let process = Process {
        cpu_time_seconds: match cpu_seconds {
            Some(seconds) => Metric::measured(
                seconds,
                "seconds",
                proc_method(
                    "stat",
                    "utime + stime of the app process during the load window",
                ),
            ),
            None => Metric::not_measured("seconds", process_reason(pid)),
        },
        peak_rss_bytes: match peak_rss {
            Some(bytes) => Metric::measured(
                bytes,
                "bytes",
                proc_method(
                    "status",
                    "VmHWM, peak resident memory since the process started",
                ),
            ),
            None => Metric::not_measured("bytes", process_reason(pid)),
        },
        idle_rss_bytes: match idle_rss {
            Some(bytes) => Metric::measured(
                bytes,
                "bytes",
                proc_method("status", "VmRSS just before the load"),
            ),
            None => Metric::not_measured("bytes", process_reason(pid)),
        },
    };
    let image = crate::generators::platform_name::package_name(Path::new("Cargo.toml"))
        .map(|name| crate::generators::platform_name::dns_label(&name));
    let artifacts = Artifacts {
        binary_size_bytes: artifacts::binary_size(
            subject.executable.as_deref(),
            subject.executable_method,
        ),
        docker_image_size_bytes: artifacts::docker_image_size(image.as_deref()),
    };
    Ok(Report {
        schema_version: report::SCHEMA_VERSION,
        cli_version: env!("CARGO_PKG_VERSION"),
        measured_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        machine: Machine::current(),
        inputs: Inputs {
            url: options.url.as_ref().map(ToString::to_string),
            path: options.path.clone(),
            duration_seconds: options.duration.as_secs_f64(),
            concurrency: options.concurrency,
            grid_intensity_g_per_kwh: options.grid_intensity,
            cpu_watts: options.cpu_watts,
            embodied_g: options.embodied,
        },
        target: subject.target.clone(),
        load: Load::from_outcome(&outcome),
        process,
        artifacts,
        energy: EnergyView::new(&energy, requests),
        carbon: sci::compute(&energy, options.grid_intensity, options.embodied, requests),
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

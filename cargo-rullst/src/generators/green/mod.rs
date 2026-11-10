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
//! measurement ran. When the build fails or the app does not start or
//! answer, the report still carries the figures that need no running app,
//! with the failure inside, and the process exits 1.

mod artifacts;
mod failure;
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
use crate::ui::error_report::ProjectRequired;
use clap::ArgMatches;
use failure::Failure;
use options::Options;
use report::{Benchmark, Machine, Measured, Metric, Report, Target};
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};

const DOCS: &str = "https://rullst.github.io/Rullst/book/green.html";

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

/// The app being measured: its URL, PID and executable, plus the process
/// started by green (stopped on drop), or why it could not be measured.
struct Subject {
    base: Option<reqwest::Url>,
    target: Target,
    executable: Option<PathBuf>,
    executable_method: &'static str,
    owned: Option<target::OwnedApp>,
    failure: Option<Failure>,
}

impl Subject {
    /// A started app (`release_build`) that green could not measure.
    fn unstarted(
        url: Option<&reqwest::Url>,
        binary: Option<PathBuf>,
        environment: &'static str,
        failure: Failure,
    ) -> Self {
        Self {
            base: None,
            target: Target {
                mode: "release_build",
                url: url.map(ToString::to_string),
                pid: None,
                process_lookup: failure.unmeasured().to_string(),
                executable: binary.as_deref().and_then(file_name),
                environment: Some(started_environment(environment)),
            },
            executable: binary,
            executable_method: "file size of the release binary built by cargo",
            owned: None,
            failure: Some(failure),
        }
    }
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
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
        let failure = Failure::reach(url.as_str());
        return Ok(Subject {
            base: None,
            target: Target {
                mode: "existing_url",
                url: Some(url.to_string()),
                pid: None,
                process_lookup: failure.unmeasured().to_string(),
                executable: None,
                environment: None,
            },
            executable: None,
            executable_method: "file size of the running executable (/proc/<pid>/exe)",
            owned: None,
            failure: Some(failure),
        });
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
        .as_deref()
        .and_then(file_name);
    Ok(Subject {
        base: Some(base.clone()),
        target: Target {
            mode: "existing_url",
            url: Some(url.to_string()),
            pid,
            process_lookup: lookup,
            executable: name,
            environment: None,
        },
        executable,
        executable_method: "file size of the running executable (/proc/<pid>/exe)",
        owned: None,
        failure: None,
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
            let failure = Failure::build(&error.to_string());
            return Ok(Subject::unstarted(None, None, environment, failure));
        }
    };
    let port = target::free_port()?;
    let base = target::validate_url(&format!("http://127.0.0.1:{port}"))?;
    let url = target::request_url(&base, path);
    eprintln!("Starting the app with RULLST_ENV={environment} on {base} ...");
    let mut app = match target::OwnedApp::spawn(&binary, port, environment) {
        Ok(app) => app,
        Err(error) => {
            let failure = Failure {
                stage: "start",
                title: "App could not be started",
                message: format!("Starting {} failed: {error}.", binary.display()),
                fix: "Check that the release binary runs: `cargo run --release`.".into(),
            };
            return Ok(Subject::unstarted(
                Some(&url),
                Some(binary),
                environment,
                failure,
            ));
        }
    };
    if let Err(error) = app.wait_ready(client, &url).await {
        let failure = Failure::start(format!("{error}."), &app.stderr_tail(), environment);
        app.stop();
        return Ok(Subject::unstarted(
            Some(&url),
            Some(binary),
            environment,
            failure,
        ));
    }
    let name = file_name(&binary);
    Ok(Subject {
        base: Some(base),
        target: Target {
            mode: "release_build",
            url: Some(url.to_string()),
            pid: Some(app.pid()),
            process_lookup: "started by green".to_string(),
            executable: name,
            environment: Some(started_environment(environment)),
        },
        executable: Some(binary),
        executable_method: "file size of the release binary built by cargo",
        owned: Some(app),
        failure: None,
    })
}

/// The variables green set for the app it started.
fn started_environment(environment: &str) -> &'static str {
    if environment == "staging" {
        "RULLST_ENV=staging HOST=127.0.0.1"
    } else {
        "RULLST_ENV=production HOST=127.0.0.1"
    }
}

async fn measure(options: &Options) -> Result<Report, Box<dyn Error>> {
    let client = load::client()?;
    let mut subject = match &options.url {
        Some(base) => existing(&client, base, &options.path).await?,
        None => spawned(&client, &options.path, options.environment).await?,
    };
    let pid = subject.target.pid;
    let idle_rss = pid.and_then(procfs::sample_rss);
    let unmeasured = subject
        .failure
        .as_ref()
        .map_or_else(|| process_reason(pid), Failure::unmeasured);
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
            Some(bytes) => Metric::measured(
                bytes,
                "bytes",
                "VmRSS after start-up, before any benchmark request (/proc/<pid>/status)",
            ),
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
        failure: subject.failure,
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

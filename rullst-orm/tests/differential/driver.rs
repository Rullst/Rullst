//! Starts one worker process per backend of this build, then compares their
//! transcripts with the SQLite transcript.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::backend::Backend;
use crate::compare;
use crate::transcript::Transcript;

const WORKER_BACKEND: &str = "RULLST_DIFF_WORKER";
const WORKER_OUTPUT: &str = "RULLST_DIFF_WORKER_OUTPUT";
const WORKER_DEADLINE: Duration = Duration::from_secs(600);

/// What a child process was started for.
pub struct WorkerJob {
    pub backend: Backend,
    pub output: PathBuf,
}

impl WorkerJob {
    pub fn from_env() -> Option<Self> {
        let backend = std::env::var(WORKER_BACKEND).ok()?;
        let backend = Backend::parse(&backend)
            .unwrap_or_else(|| panic!("{WORKER_BACKEND}={backend} names no backend"));
        let output =
            std::env::var_os(WORKER_OUTPUT).unwrap_or_else(|| panic!("{WORKER_OUTPUT} is not set"));
        Some(Self {
            backend,
            output: PathBuf::from(output),
        })
    }
}

/// Whether CI demands every backend of this build and the SQLite reference.
fn required() -> bool {
    std::env::var("RULLST_DIFF_REQUIRE_ALL").is_ok_and(|value| value == "1")
}

fn transcript_dir() -> PathBuf {
    std::env::var_os("RULLST_DIFF_DIR").map_or_else(
        || Path::new(env!("CARGO_TARGET_TMPDIR")).join("rullst-differential"),
        PathBuf::from,
    )
}

pub fn run() {
    let dir = transcript_dir();
    std::fs::create_dir_all(&dir).expect("create the transcript directory");
    let reference = dir.join("sqlite.json");
    let mut problems = Vec::new();

    for &backend in Backend::compiled() {
        if backend == Backend::Sqlite {
            // The reference, run twice: identical transcripts prove that no
            // step records a time-, id- or order-dependent value by accident.
            let repeat = dir.join("sqlite-repeat.json");
            spawn_worker(backend, &reference);
            spawn_worker(backend, &repeat);
            problems.extend(compare::identical(
                &Transcript::read(&reference),
                &Transcript::read(&repeat),
            ));
            continue;
        }
        let variable = backend.url_variable().expect("server backends have a URL");
        if std::env::var_os(variable).is_none() {
            assert!(
                !required(),
                "RULLST_DIFF_REQUIRE_ALL=1 but {variable} is not set"
            );
            eprintln!("skipping {}: {variable} is not set", backend.name());
            continue;
        }
        let output = dir.join(format!("{}.json", backend.name()));
        spawn_worker(backend, &output);
        if !reference.exists() {
            assert!(
                !required(),
                "RULLST_DIFF_REQUIRE_ALL=1 but {} is missing; run the strict-sqlite build first",
                reference.display()
            );
            eprintln!(
                "not comparing {}: {} is missing; run the strict-sqlite build first",
                backend.name(),
                reference.display()
            );
            continue;
        }
        let report =
            compare::against_reference(&Transcript::read(&reference), &Transcript::read(&output));
        let path = dir.join(format!("{}-comparison.txt", backend.name()));
        std::fs::write(&path, report.text()).expect("write the comparison report");
        println!("{}", report.text());
        problems.extend(report.problems);
    }

    assert!(
        problems.is_empty(),
        "{} undocumented difference(s):\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// Runs the `worker` test of this binary for `backend` and waits for it.
fn spawn_worker(backend: Backend, output: &Path) {
    let _ = std::fs::remove_file(output);
    let mut child = Command::new(std::env::current_exe().expect("test binary"))
        .args(["worker", "--exact", "--nocapture", "--test-threads=1"])
        .env(WORKER_BACKEND, backend.name())
        .env(WORKER_OUTPUT, output)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("start the worker process");
    let deadline = Instant::now() + WORKER_DEADLINE;
    let status = loop {
        if let Some(status) = child.try_wait().expect("worker status") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!(
                "the {} worker did not finish within {WORKER_DEADLINE:?}",
                backend.name()
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(
        status.success(),
        "the {} worker failed: {status}",
        backend.name()
    );
    assert!(
        output.exists(),
        "the {} worker wrote no transcript",
        backend.name()
    );
}

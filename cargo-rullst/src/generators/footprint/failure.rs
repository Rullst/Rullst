//! Why the app could not be measured: the release build failed, the started
//! app never answered, or nothing answered on `--url`. The report still
//! carries the figures that need no running app (binary size when a binary
//! was built, image size, dependency count) together with this failure.
use crate::ui::error_report::{self, AlreadyReported, Friendly};
use serde::Serialize;
use std::error::Error;

/// Lines and characters of a failed build's log shown in the report.
const EXCERPT_LINES: usize = 20;
const EXCERPT_CHARS: usize = 2000;
/// Cargo's progress lines, which say nothing about the failure.
const PROGRESS: &[&str] = &[
    "Adding",
    "Blocking",
    "Building",
    "Checking",
    "Compiling",
    "Downloaded",
    "Downloading",
    "Fetching",
    "Finished",
    "Fresh",
    "Locking",
    "Packaging",
    "Removing",
    "Running",
    "Unpacking",
    "Updated",
    "Updating",
    "Waiting",
];

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Failure {
    /// `build`, `start` or `reach`.
    pub stage: &'static str,
    pub title: &'static str,
    pub message: String,
    pub fix: String,
}

impl Failure {
    pub(super) fn build(log: &str) -> Self {
        Self {
            stage: "build",
            title: "Release build failed",
            message: error_report::sanitize(&build_excerpt(log)),
            fix: "Fix the build errors (`cargo build --release`), then run footprint again.".into(),
        }
    }

    /// The started app exited or never answered; `stderr` is its last lines.
    pub(super) fn start(happened: String, stderr: &str, environment: &str) -> Self {
        let message = if stderr.is_empty() {
            happened
        } else {
            format!("{happened}\nLast stderr lines:\n{stderr}")
        };
        Self {
            stage: "start",
            title: "App could not be started",
            fix: start_fix(environment, &message).into(),
            message,
        }
    }

    pub(super) fn reach(url: &str) -> Self {
        Self {
            stage: "reach",
            title: "App not reachable",
            message: format!("Nothing answered on {url}."),
            fix: "Start the app first (for example `cargo rullst dev` or the release binary), or omit --url to let footprint build and start it.".into(),
        }
    }

    /// The reason given for the figures that needed the running app.
    pub(super) fn unmeasured(&self) -> &'static str {
        match self.stage {
            "build" => "the release build failed",
            "start" => "the app did not start",
            _ => "the app was not reachable",
        }
    }

    /// Prints the friendly report on stderr; the exit status is 1.
    pub(super) fn report(&self) -> Box<dyn Error> {
        let friendly = Friendly {
            title: self.title.to_string(),
            happened: self.message.clone(),
            fix: Some(self.fix.clone()),
            docs: Some(super::DOCS),
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
        AlreadyReported::new(1, self.message.clone()).into()
    }
}

fn progress(line: &str) -> bool {
    let trimmed = line.trim_start();
    PROGRESS.iter().any(|verb| {
        trimmed
            .strip_prefix(verb)
            .is_some_and(|rest| rest.starts_with(' '))
    })
}

/// The part of a failed build's log that explains it: without Cargo's
/// progress lines, from the first error (warnings before it are dropped),
/// and at most [`EXCERPT_LINES`] lines and [`EXCERPT_CHARS`] characters.
pub(super) fn build_excerpt(log: &str) -> String {
    let lines = log
        .lines()
        .filter(|line| !progress(line) && !line.trim().is_empty())
        .collect::<Vec<_>>();
    let first_error = lines
        .iter()
        .position(|line| line.trim_start().starts_with("error"))
        .unwrap_or(0);
    let relevant = &lines[first_error..];
    let mut excerpt = String::new();
    let mut shown = 0;
    for line in relevant.iter().take(EXCERPT_LINES) {
        if excerpt.chars().count() + line.chars().count() > EXCERPT_CHARS {
            break;
        }
        if shown > 0 {
            excerpt.push('\n');
        }
        excerpt.push_str(line);
        shown += 1;
    }
    if shown < relevant.len() {
        excerpt.push_str(&format!(
            "\n… {} more line(s); run `cargo build --release` for the whole log",
            relevant.len() - shown
        ));
    }
    excerpt
}

/// What to do when the started app in `environment` never answered, from
/// what it reported (`happened`, with its last stderr lines).
pub(super) fn start_fix(environment: &str, happened: &str) -> &'static str {
    let text = happened.to_ascii_lowercase();
    let says = |words: &[&str]| words.iter().any(|word| text.contains(word));
    if says(&[
        "database",
        "pool timed out",
        "sqlx",
        "no such table",
        "sqlite",
        "postgres",
        "mysql",
    ]) {
        "The app could not use its database: start the database server and check DATABASE_URL (environment, .env or Rullst.toml [database] url), run `cargo rullst db:migrate` against it, or start the app yourself and measure it with --url."
    } else if says(&["app_key"]) {
        "Set APP_KEY (32 or more random characters) in the environment or .env.production; production refuses the development key. Or measure with --env staging."
    } else if says(&["billing", "stripe"]) {
        "Production needs real billing credentials (the SaaS starter's Stripe keys, for example): set them, measure with --env staging, which accepts the starters' mock credentials, or start the app yourself and measure it with --url."
    } else if environment == "production" {
        "Production needs the app's production settings: set the ones the error above names, measure with --env staging, or start the app yourself and measure it with --url."
    } else {
        "Run the release binary with RULLST_ENV=staging to see why it does not serve requests, or measure a running app with --url."
    }
}

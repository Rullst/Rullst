//! The Rullst/Benchmarks site summary (`results/site.json`, schema `rullst.benchmarks.site.v2`):
//! every value checked before it reaches a page, and rendered into the comparison pages'
//! Performance section and the landing's proof point and summary.
//!
//! An unknown schema, category, flag, status, stability result, role or weighting, a bad or
//! future date, results older than the age limit, a non-finite or negative number, an
//! unexpected version string, another reference than Axum or a run URL outside
//! Rullst/Benchmarks discards the whole file: the pages keep their pending text rather than
//! show older or partially understood numbers.

mod check;
pub mod compare;
mod format;
pub mod landing;
mod text;

use std::collections::BTreeSet;
use std::path::Path;

use crate::date::Date;

/// The fixed name `--benchmarks` reads from the working directory; never a results path.
pub const FILE: &str = "benchmarks.json";
pub const SCHEMA: &str = "rullst.benchmarks.site.v2";
/// Results measured longer ago than this are not shown (`--benchmarks-max-age-days`).
pub const MAX_AGE_DAYS: i64 = 90;
/// "rullst" is the application `cargo rullst new` generates, which has a position;
/// "rullst-minimal" is the framework without that application's middleware, a
/// diagnostic row without a position, shown only when the data has it.
pub const MINIMAL: &str = "rullst-minimal";
/// The rows shown (Rullst/Benchmarks slug, name), in their fixed order.
pub const ROWS: [(&str, &str); 6] = [
    ("rullst", "Rullst"),
    ("loco", "Loco"),
    ("topcoat", "Topcoat"),
    ("actix", "Actix Web"),
    ("gin", "Gin"),
    (MINIMAL, "Rullst (minimal)"),
];
/// The data must use exactly these categories, overall weights and flags; the pages
/// explain no other (a new one needs a new schema and a change here).
pub const CATEGORIES: [&str; 5] = ["hello", "database", "application", "memory", "startup"];
pub const WEIGHTS: [(&str, i64); 5] = [
    ("hello", 2),
    ("database", 2),
    ("application", 2),
    ("memory", 1),
    ("startup", 1),
];
/// Each flag and its sign, in the order the pages write them.
pub const FLAGS: [(&str, &str); 4] = [
    ("cpu_cost", "*"),
    ("database_note", "†"),
    ("put_not_written", "‡"),
    ("within_reference_margin", "≈"),
];
pub const STATUSES: [&str; 7] = [
    "ranked",
    "diagnostic",
    "errors",
    "incomplete",
    "not comparable",
    "not measured",
    "unstable",
];
pub const STABILITY: [&str; 3] = ["pass", "warn", "fail"];
pub const MEMORY: [&str; 4] = [
    "peak_ram_mib",
    "axum_peak_ram_mib",
    "after_logins_peak_ram_mib",
    "axum_after_logins_peak_ram_mib",
];
/// The categories whose scores are throughput (the "speed" of the proof point).
pub const SPEED: [&str; 3] = ["hello", "database", "application"];

/// One score: its status, value, position range, tie margin and known flags.
#[derive(Clone, Debug)]
pub struct Cell {
    pub status: &'static str,
    pub score: Option<f64>,
    pub position: Option<(u32, u32)>,
    pub margin: Option<f64>,
    pub flags: BTreeSet<&'static str>,
}

impl Cell {
    pub fn has(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }
}

/// One shown implementation.
#[derive(Clone, Debug)]
pub struct Row {
    pub slug: &'static str,
    pub version: String,
    /// Present exactly when the run publishes an overall score.
    pub overall: Option<Cell>,
    /// In the order of [`CATEGORIES`].
    pub categories: Vec<Cell>,
    pub stability: Option<&'static str>,
    /// Measured memory figures, keyed by the names in [`MEMORY`].
    pub memory: Vec<(&'static str, f64)>,
}

impl Row {
    pub fn category(&self, key: &str) -> Option<&Cell> {
        CATEGORIES
            .iter()
            .position(|category| *category == key)
            .and_then(|index| self.categories.get(index))
    }

    pub fn memory(&self, key: &str) -> Option<f64> {
        self.memory
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| *value)
    }

    /// Every cell of the row: the overall score, then the categories.
    pub fn cells(&self) -> impl Iterator<Item = &Cell> {
        self.overall.iter().chain(self.categories.iter())
    }
}

/// Every value the pages show, checked.
#[derive(Clone, Debug)]
pub struct Results {
    pub measured_on: Date,
    pub rullst_version: String,
    /// "crates.io" or "git"; another source is left out of the meta line.
    pub source: Option<&'static str>,
    pub run_url: String,
    pub run_id: String,
    pub axum_version: String,
    pub ranked: u32,
    pub overall_published: bool,
    pub logins: Option<u32>,
    /// The shown rows in the order of the data.
    pub rows: Vec<Row>,
    /// The English and Portuguese notes, when the data has them.
    pub note_en: Option<String>,
    pub note_pt: Option<String>,
    pub reading_en: String,
    pub reading_pt: String,
}

impl Results {
    pub fn row(&self, slug: &str) -> Option<&Row> {
        self.rows.iter().find(|row| row.slug == slug)
    }
}

/// The name a row has in the data's English (the comparison's names are the same in both languages
/// apart from the minimal row, which the pages translate).
pub fn row_name(slug: &str) -> &'static str {
    ROWS.iter()
        .find(|(known, _)| *known == slug)
        .map_or("", |(_, name)| name)
}

/// Returns the checked results, or None after a warning.
pub fn load(path: &Path, today: Date, max_age_days: i64) -> Option<Results> {
    let results =
        crate::input::read_json(path).and_then(|data| check::results(&data, today, max_age_days));
    match results {
        Err(error) => {
            eprintln!("warning: benchmark results not used ({error})");
            None
        }
        Ok(results) if results.rows.is_empty() => {
            eprintln!("warning: benchmark results name none of the compared frameworks");
            None
        }
        Ok(results) => Some(results),
    }
}

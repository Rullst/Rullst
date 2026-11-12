//! Footprint budgets for CI: limits on the measured figures, read from the
//! `[footprint.budgets]` table of `Rullst.toml` (`--check-budget`) or of
//! another TOML file (`--budget FILE`). Each limit is checked against its
//! figure; a limit whose figure was not measured is **skipped**, never
//! passed, and any exceeded limit makes the command exit with status 1.

use super::render::{bytes, number};
use super::report::{Metric, Report};
use crate::term::Style;
use crate::ui::error_report::{self, AlreadyReported, Friendly};
use serde::Serialize;
use std::error::Error;
use std::path::{Path, PathBuf};

/// Larger budget files are not read.
const MAX_FILE_BYTES: u64 = 1024 * 1024;
/// The project configuration that `--check-budget` reads.
pub(super) const RULLST_TOML: &str = "Rullst.toml";

/// How a limit's figure is counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Unit {
    Bytes,
    Count,
    Milliseconds,
}

impl Unit {
    fn as_str(self) -> &'static str {
        match self {
            Self::Bytes => "bytes",
            Self::Count => "count",
            Self::Milliseconds => "milliseconds",
        }
    }

    fn show(self, value: f64) -> String {
        match self {
            Self::Bytes => bytes(value.round() as u64),
            Self::Count => format!("{}", value.round() as u64),
            Self::Milliseconds => format!("{} ms", number(value)),
        }
    }
}

/// Every limit a budget can set, in report order, with its unit.
pub(super) const LIMITS: [(&str, Unit); 10] = [
    ("binary_size_bytes", Unit::Bytes),
    ("image_size_bytes", Unit::Bytes),
    ("idle_rss_bytes", Unit::Bytes),
    ("dependency_count", Unit::Count),
    ("cpu_ms_per_request", Unit::Milliseconds),
    ("bytes_per_request", Unit::Bytes),
    ("page_first_visit_bytes", Unit::Bytes),
    ("page_repeat_visit_bytes", Unit::Bytes),
    ("page_requests", Unit::Count),
    ("page_third_party_origins", Unit::Count),
];

/// Where a budget comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Source {
    /// `[footprint.budgets]` of `Rullst.toml` in the current directory.
    RullstToml,
    /// `[footprint.budgets]` of this file.
    File(PathBuf),
}

/// The limits of one budget, in [`LIMITS`] order.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Budget {
    source: String,
    limits: Vec<(&'static str, Unit, f64)>,
}

/// A budget that cannot be used; the run stops before measuring.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum BudgetError {
    #[error("{0} could not be read ({1})")]
    Read(String, String),
    #[error("{0} is larger than 1 MiB")]
    TooLarge(String),
    #[error("{0} is not valid TOML at {1}")]
    Syntax(String, String),
    #[error("{0} has no [footprint.budgets] table")]
    Missing(String),
    #[error("[footprint.budgets] in {0} sets no limit")]
    Empty(String),
    #[error("[footprint.budgets] in {file}: `{key}` is not a limit; the limits are {known}")]
    Unknown {
        file: String,
        key: String,
        known: String,
    },
    #[error("[footprint.budgets] in {file}: `{key}` must be a number of 0 or more")]
    Invalid { file: String, key: String },
}

/// Reads the budget of `source`, relative to `root`.
pub(super) fn load(source: &Source, root: &Path) -> Result<Budget, BudgetError> {
    let (path, name) = match source {
        Source::RullstToml => (root.join(RULLST_TOML), RULLST_TOML.to_string()),
        Source::File(path) => (root.join(path), path.display().to_string()),
    };
    let metadata = std::fs::metadata(&path)
        .map_err(|error| BudgetError::Read(name.clone(), error.kind().to_string()))?;
    if metadata.len() > MAX_FILE_BYTES {
        return Err(BudgetError::TooLarge(name));
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|error| BudgetError::Read(name.clone(), error.kind().to_string()))?;
    parse(&text, &name)
}

/// The budget in `text`, a TOML document called `name` in messages. Values
/// never appear in a message, so a file holding secrets cannot leak them.
pub(super) fn parse(text: &str, name: &str) -> Result<Budget, BudgetError> {
    let document: toml::Value = toml::from_str(text).map_err(|error| {
        BudgetError::Syntax(
            name.to_string(),
            crate::generators::toml_error_position(text, &error),
        )
    })?;
    let table = document
        .get("footprint")
        .and_then(|footprint| footprint.get("budgets"))
        .and_then(toml::Value::as_table)
        .ok_or_else(|| BudgetError::Missing(name.to_string()))?;
    let mut limits = Vec::new();
    for (key, value) in table {
        let Some((limit, unit)) = LIMITS.iter().find(|(limit, _)| limit == key) else {
            return Err(BudgetError::Unknown {
                file: name.to_string(),
                key: crate::term::sanitize(key),
                known: LIMITS.map(|(limit, _)| limit).join(", "),
            });
        };
        let allowed = value
            .as_integer()
            .map(|integer| integer as f64)
            .or_else(|| value.as_float())
            .filter(|allowed| allowed.is_finite() && *allowed >= 0.0)
            .ok_or_else(|| BudgetError::Invalid {
                file: name.to_string(),
                key: key.clone(),
            })?;
        limits.push((*limit, *unit, allowed));
    }
    if limits.is_empty() {
        return Err(BudgetError::Empty(name.to_string()));
    }
    limits.sort_by_key(|(limit, _, _)| LIMITS.iter().position(|(known, _)| known == limit));
    Ok(Budget {
        source: name.to_string(),
        limits,
    })
}

/// One limit against its figure.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct LimitResult {
    pub name: &'static str,
    /// `bytes`, `count` or `milliseconds`.
    pub unit: &'static str,
    pub allowed: f64,
    /// `null` when the figure was not measured.
    pub measured: Option<f64>,
    /// `passed`, `exceeded` or `skipped` (not measured).
    pub status: &'static str,
    /// Why a skipped limit's figure was not measured.
    pub reason: Option<String>,
}

/// The budget section of the report.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Budgets {
    /// `not_checked` (no budget asked for), `passed`, `exceeded` or
    /// `skipped` (no limit's figure was measured).
    pub status: &'static str,
    pub source: Option<String>,
    pub passed: u64,
    pub exceeded: u64,
    pub skipped: u64,
    pub limits: Vec<LimitResult>,
}

impl Budgets {
    pub(super) fn not_checked() -> Self {
        Self {
            status: "not_checked",
            source: None,
            passed: 0,
            exceeded: 0,
            skipped: 0,
            limits: Vec::new(),
        }
    }
}

fn metric<T: Copy + Into<f64>>(metric: &Metric<T>) -> Result<f64, String> {
    metric.value.map(Into::into).ok_or_else(|| {
        metric
            .reason
            .clone()
            .unwrap_or_else(|| "not measured".into())
    })
}

fn counted(metric: &Metric<u64>) -> Result<f64, String> {
    metric.value.map(|value| value as f64).ok_or_else(|| {
        metric
            .reason
            .clone()
            .unwrap_or_else(|| "not measured".into())
    })
}

/// The figure limit `name` bounds, or why it was not measured.
pub(super) fn figure(name: &str, report: &Report) -> Result<f64, String> {
    let page = &report.page_weight;
    let page_reason = || page.reason.clone().unwrap_or_else(|| "not measured".into());
    let page_figure = |value: Option<u64>| value.map(|value| value as f64).ok_or_else(page_reason);
    match name {
        "binary_size_bytes" => counted(&report.measured.binary_size_bytes),
        "image_size_bytes" => counted(&report.measured.image_size_bytes),
        "idle_rss_bytes" => counted(&report.measured.idle_rss_bytes),
        "dependency_count" => counted(&report.measured.dependency_count),
        "cpu_ms_per_request" => metric(&report.benchmark.cpu_ms_per_request),
        "bytes_per_request" => metric(&report.benchmark.bytes_per_request),
        "page_first_visit_bytes" => page_figure(page.first_visit.bytes),
        "page_repeat_visit_bytes" => page_figure(page.repeat_visit.bytes),
        "page_requests" => page_figure(page.first_visit.requests),
        "page_third_party_origins" if page.status == "measured" => {
            Ok(page.third_party_origins.len() as f64)
        }
        "page_third_party_origins" => Err(page_reason()),
        _ => Err("not a limit".into()),
    }
}

/// Every limit of `budget` against `report`.
pub(super) fn check(budget: &Budget, report: &Report) -> Budgets {
    let mut checked = Budgets {
        status: "skipped",
        source: Some(budget.source.clone()),
        ..Budgets::not_checked()
    };
    for &(name, unit, allowed) in &budget.limits {
        let (status, measured, reason) = match figure(name, report) {
            Ok(measured) if measured > allowed => ("exceeded", Some(measured), None),
            Ok(measured) => ("passed", Some(measured), None),
            Err(reason) => ("skipped", None, Some(reason)),
        };
        match status {
            "exceeded" => checked.exceeded += 1,
            "passed" => checked.passed += 1,
            _ => checked.skipped += 1,
        }
        checked.limits.push(LimitResult {
            name,
            unit: unit.as_str(),
            allowed,
            measured,
            status,
            reason,
        });
    }
    checked.status = if checked.exceeded > 0 {
        "exceeded"
    } else if checked.passed > 0 {
        "passed"
    } else {
        "skipped"
    };
    checked
}

fn unit_of(result: &LimitResult) -> Unit {
    LIMITS
        .iter()
        .find(|(name, _)| *name == result.name)
        .map_or(Unit::Count, |(_, unit)| *unit)
}

/// `measured > allowed` of an exceeded limit, in its unit.
fn comparison(result: &LimitResult) -> String {
    let unit = unit_of(result);
    let measured = result
        .measured
        .map_or_else(String::new, |value| unit.show(value));
    match result.status {
        "exceeded" => format!("{measured} > {} allowed", unit.show(result.allowed)),
        "passed" => format!("{measured} of {} allowed", unit.show(result.allowed)),
        _ => format!(
            "not measured: {}",
            result.reason.as_deref().unwrap_or("not measured")
        ),
    }
}

/// The budget section of the terminal report; empty without a budget.
pub(super) fn render(budgets: &Budgets, style: Style) -> String {
    if budgets.status == "not_checked" {
        return String::new();
    }
    let title = format!(
        "Budgets ({} [footprint.budgets])",
        budgets.source.as_deref().unwrap_or(RULLST_TOML)
    );
    let mut out = format!("{}\n", style.bold(&title));
    for result in &budgets.limits {
        let label = match result.status {
            "exceeded" => style.yellow("EXCEEDED"),
            "passed" => "passed  ".to_string(),
            _ => style.yellow("SKIPPED "),
        };
        out.push_str(&format!(
            "  {label}  {:<25} {}\n",
            result.name,
            comparison(result)
        ));
    }
    out.push_str(&format!(
        "  {}\n\n",
        style.dim(&format!(
            "{} passed, {} exceeded, {} skipped (a skipped limit was not measured, so it did not pass)",
            budgets.passed, budgets.exceeded, budgets.skipped
        ))
    ));
    out
}

/// Prints the exceeded limits on stderr; the returned error exits with 1.
pub(super) fn exceeded_report(budgets: &Budgets) -> Box<dyn Error> {
    let exceeded: Vec<String> = budgets
        .limits
        .iter()
        .filter(|result| result.status == "exceeded")
        .map(|result| format!("{}: {}", result.name, comparison(result)))
        .collect();
    let friendly = Friendly {
        title: "Footprint budget exceeded".to_string(),
        happened: exceeded.join("\n"),
        fix: Some(format!(
            "Bring the figures under their limits, or raise the limits in {} when the increase is intended.",
            budgets.source.as_deref().unwrap_or(RULLST_TOML)
        )),
        docs: Some(super::BUDGET_DOCS),
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
    AlreadyReported::new(1, format!("{} footprint limit(s) exceeded", exceeded.len())).into()
}

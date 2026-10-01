//! The doctor's result model and its versioned JSON form
//! (`rullst.cli-doctor.v1`, documented in the CLI reference).

use serde::Serialize;

pub(crate) const SCHEMA_VERSION: &str = "rullst.cli-doctor.v1";
const DOCS_BASE: &str = "https://rullst.github.io/Rullst/book/cli_reference.html#doctor-";

/// The outcome of one check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Status {
    /// ✓ Healthy.
    Pass,
    /// ! Works, but needs attention.
    Warn,
    /// ✗ Broken; the doctor exits with status 1.
    Fail,
    /// · Informational, optional or not applicable.
    Info,
}

impl Status {
    pub(crate) const fn glyph(self) -> &'static str {
        match self {
            Self::Pass => "✓",
            Self::Warn => "!",
            Self::Fail => "✗",
            Self::Info => "·",
        }
    }
}

/// One diagnostic line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Check {
    pub id: &'static str,
    pub title: &'static str,
    pub status: Status,
    pub detail: String,
    /// A one-line remedy; always present for `warn` and `fail`.
    pub fix: Option<String>,
    /// Documentation for `warn` and `fail`, filled in by [`Report::new`].
    pub docs: Option<String>,
}

impl Check {
    fn new(id: &'static str, title: &'static str, status: Status, detail: String) -> Self {
        Self {
            id,
            title,
            status,
            detail,
            fix: None,
            docs: None,
        }
    }

    pub(crate) fn pass(id: &'static str, title: &'static str, detail: impl Into<String>) -> Self {
        Self::new(id, title, Status::Pass, detail.into())
    }

    pub(crate) fn info(id: &'static str, title: &'static str, detail: impl Into<String>) -> Self {
        Self::new(id, title, Status::Info, detail.into())
    }

    pub(crate) fn warn(
        id: &'static str,
        title: &'static str,
        detail: impl Into<String>,
        fix: impl Into<String>,
    ) -> Self {
        Self::new(id, title, Status::Warn, detail.into()).with_fix(fix)
    }

    pub(crate) fn fail(
        id: &'static str,
        title: &'static str,
        detail: impl Into<String>,
        fix: impl Into<String>,
    ) -> Self {
        Self::new(id, title, Status::Fail, detail.into()).with_fix(fix)
    }

    pub(crate) fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }
}

/// A titled set of checks; `anchor` names its section in the CLI reference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Group {
    pub id: &'static str,
    pub title: &'static str,
    #[serde(skip)]
    pub anchor: &'static str,
    pub checks: Vec<Check>,
}

impl Group {
    pub(crate) fn new(
        id: &'static str,
        title: &'static str,
        anchor: &'static str,
        checks: Vec<Check>,
    ) -> Self {
        Self {
            id,
            title,
            anchor,
            checks,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub(crate) struct Summary {
    pub pass: usize,
    pub warn: usize,
    pub fail: usize,
    pub info: usize,
}

/// Everything the doctor found, in display order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Report {
    pub schema_version: &'static str,
    pub cli_version: &'static str,
    /// `false` when any check failed.
    pub ok: bool,
    pub summary: Summary,
    pub groups: Vec<Group>,
}

impl Report {
    pub(crate) fn new(mut groups: Vec<Group>) -> Self {
        let mut summary = Summary::default();
        for group in &mut groups {
            for check in &mut group.checks {
                match check.status {
                    Status::Pass => summary.pass += 1,
                    Status::Warn => summary.warn += 1,
                    Status::Fail => summary.fail += 1,
                    Status::Info => summary.info += 1,
                }
                if matches!(check.status, Status::Warn | Status::Fail) && check.docs.is_none() {
                    check.docs = Some(format!("{DOCS_BASE}{}", group.anchor));
                }
            }
        }
        Self {
            schema_version: SCHEMA_VERSION,
            cli_version: env!("CARGO_PKG_VERSION"),
            ok: summary.fail == 0,
            summary,
            groups,
        }
    }

    /// The check with `id`, for tests and the `--fix` follow-up.
    #[cfg(test)]
    pub(crate) fn check(&self, id: &str) -> Option<&Check> {
        self.groups
            .iter()
            .flat_map(|group| &group.checks)
            .find(|check| check.id == id)
    }
}

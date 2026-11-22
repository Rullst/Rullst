//! Which rules a plan applies: by default those introduced after the Rullst
//! version the application uses, or after the version passed with `--from`.

use super::{CATALOG, Rule, SourceScan};
use semver::Version;

/// The rules a plan applies: by default only those introduced after the
/// Rullst version the application already uses, or after the version passed
/// with `--from`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RuleSelection {
    /// The oldest Rullst version the manifests require or `Cargo.lock` holds.
    pub current: Option<Version>,
    /// The version passed with `--from`, which replaces `current` as the
    /// version whose later rules apply.
    pub from: Option<Version>,
    /// Every rule applies, whatever `current` is (`--all-rules`).
    pub all_rules: bool,
    /// Catalog rules introduced in or before `from` (else `current`), not
    /// applied.
    pub skipped_rules: usize,
    /// Findings of the skipped rules, left out of the plan.
    pub hidden_findings: usize,
}

impl RuleSelection {
    /// The version whose later rules apply: `from`, else `current`; `None`
    /// when every rule applies anyway.
    pub(crate) fn baseline(&self) -> Option<&Version> {
        self.from.as_ref().or(self.current.as_ref())
    }
}

/// The latest release that introduced catalog rules before `version`, the
/// release an application on `version` most likely upgraded from.
pub(crate) fn release_before(version: &Version) -> Option<Version> {
    CATALOG
        .iter()
        .filter_map(|rule| Version::parse(rule.introduced).ok())
        .filter(|introduced| introduced.cmp_precedence(version).is_lt())
        .max_by(|left, right| left.cmp_precedence(right))
}

impl SourceScan {
    /// Keeps the findings of rules introduced after `from`, else after
    /// `current` (all of them with `all_rules` or an unknown version), and
    /// records what was skipped.
    pub(crate) fn select(
        &mut self,
        current: Option<Version>,
        from: Option<Version>,
        all_rules: bool,
    ) {
        let baseline = from.clone().or_else(|| current.clone());
        let applies = |rule: &Rule| {
            all_rules
                || baseline
                    .as_ref()
                    .is_none_or(|baseline| rule.introduced_after(baseline))
        };
        let found = self.findings.len();
        self.findings.retain(|finding| applies(finding.rule));
        self.reviewed.retain(|rule| applies(rule));
        self.selection = RuleSelection {
            skipped_rules: CATALOG.iter().filter(|rule| !applies(rule)).count(),
            hidden_findings: found - self.findings.len(),
            current,
            from,
            all_rules,
        };
    }
}

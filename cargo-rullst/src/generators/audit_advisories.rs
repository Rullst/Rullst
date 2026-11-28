//! The advisories listed in a `cargo audit --json` report, so the audit
//! output names each advisory, the affected crate and version, and the fixed
//! versions without a second `cargo audit` run.

use crate::ui::error_report::sanitize;

/// At most this many advisories are listed; the rest are counted.
pub(super) const LISTED_ADVISORIES: usize = 50;

/// One entry of the report's `vulnerabilities.list`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ReportedAdvisory {
    pub id: String,
    pub title: String,
    pub package: String,
    pub version: String,
    /// The `versions.patched` requirements, such as `>=2.5.9`; empty when the
    /// advisory lists no fixed version.
    pub patched: Vec<String>,
}

impl ReportedAdvisory {
    /// `RUSTSEC-2026-0592 in url 2.5.8: <title>; fixed in >=2.5.9`, sanitized
    /// (the text comes from the advisory database).
    pub(super) fn summary(&self) -> String {
        let fixed = if self.patched.is_empty() {
            "no fixed version is listed".to_string()
        } else {
            format!("fixed in {}", self.patched.join(" or "))
        };
        let title = if self.title.is_empty() {
            String::new()
        } else {
            format!(": {}", self.title)
        };
        let affected = if self.version.is_empty() {
            self.package.clone()
        } else {
            format!("{} {}", self.package, self.version)
        };
        // One line: a newline or tab in the database text would add lines to
        // the listing.
        let line = format!("{} in {affected}{title}; {fixed}", self.id);
        sanitize(&line.split_whitespace().collect::<Vec<_>>().join(" "))
    }
}

/// The advisories of a `cargo audit --json` report; empty when the output is
/// not such a report or lists none.
pub(super) fn reported_advisories(stdout: &[u8]) -> Vec<ReportedAdvisory> {
    let Ok(report) = serde_json::from_slice::<serde_json::Value>(stdout) else {
        return Vec::new();
    };
    let Some(list) = report
        .get("vulnerabilities")
        .and_then(|vulnerabilities| vulnerabilities.get("list"))
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    list.iter().filter_map(advisory).collect()
}

fn advisory(entry: &serde_json::Value) -> Option<ReportedAdvisory> {
    let text = |value: Option<&serde_json::Value>| {
        value
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let advisory = entry.get("advisory")?;
    let id = text(advisory.get("id"));
    if id.is_empty() {
        return None;
    }
    let package = entry.get("package");
    let patched = entry
        .get("versions")
        .and_then(|versions| versions.get("patched"))
        .and_then(serde_json::Value::as_array)
        .map(|requirements| {
            requirements
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    // The advisory names the crate too, for a report without the package entry.
    let name = text(package.and_then(|package| package.get("name")));
    Some(ReportedAdvisory {
        id,
        title: text(advisory.get("title")),
        package: if name.is_empty() {
            text(advisory.get("package"))
        } else {
            name
        },
        version: text(package.and_then(|package| package.get("version"))),
        patched,
    })
}

/// The lines listing `advisories` out of `count` reported: one summary per
/// advisory up to [`LISTED_ADVISORIES`], then how many were not listed.
pub(super) fn listing(advisories: &[ReportedAdvisory], count: usize) -> Vec<String> {
    let mut lines: Vec<String> = advisories
        .iter()
        .take(LISTED_ADVISORIES)
        .map(ReportedAdvisory::summary)
        .collect();
    let unlisted = count.max(advisories.len()).saturating_sub(lines.len());
    if !lines.is_empty() && unlisted > 0 {
        lines.push(format!(
            "and {unlisted} more; run `cargo audit` for the full list"
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::{LISTED_ADVISORIES, ReportedAdvisory, listing, reported_advisories};

    const REPORT: &str = r#"{"vulnerabilities":{"found":true,"count":2,"list":[
        {"advisory":{"id":"RUSTSEC-2026-0592","package":"url","title":"Host confusion in `url`"},
         "versions":{"patched":[">=2.5.9"],"unaffected":[]},
         "package":{"name":"url","version":"2.5.8"}},
        {"advisory":{"id":"RUSTSEC-2099-0002","package":"old","title":""},
         "versions":{"patched":[],"unaffected":[]},
         "package":{"name":"old","version":"0.1.0"}}
    ]},"warnings":{}}"#;

    #[test]
    fn each_advisory_names_the_crate_version_and_fix() {
        let advisories = reported_advisories(REPORT.as_bytes());
        assert_eq!(advisories.len(), 2);
        assert_eq!(
            advisories[0].summary(),
            "RUSTSEC-2026-0592 in url 2.5.8: Host confusion in `url`; fixed in >=2.5.9"
        );
        assert_eq!(
            advisories[1].summary(),
            "RUSTSEC-2099-0002 in old 0.1.0; no fixed version is listed"
        );
    }

    #[test]
    fn several_fixed_ranges_and_a_missing_package_entry() {
        let report = r#"{"vulnerabilities":{"count":1,"list":[
            {"advisory":{"id":"RUSTSEC-2099-0003","package":"dual","title":"t"},
             "versions":{"patched":[">=1.2.3, <2.0.0",">=2.1.0"]}}
        ]}}"#;
        let advisories = reported_advisories(report.as_bytes());
        assert_eq!(
            advisories[0].summary(),
            "RUSTSEC-2099-0003 in dual: t; fixed in >=1.2.3, <2.0.0 or >=2.1.0"
        );
    }

    #[test]
    fn output_that_is_not_a_report_lists_nothing() {
        assert!(reported_advisories(b"not json").is_empty());
        assert!(reported_advisories(br#"{"vulnerabilities":{"count":1}}"#).is_empty());
        // An entry without an advisory id is skipped.
        let report = r#"{"vulnerabilities":{"list":[{"advisory":{"title":"x"}}]}}"#;
        assert!(reported_advisories(report.as_bytes()).is_empty());
    }

    #[test]
    fn database_text_is_sanitized() {
        let advisory = ReportedAdvisory {
            id: "RUSTSEC-2099-0004".into(),
            title: "bad\u{1b}[31m\n  title".into(),
            package: "p".into(),
            version: "1.0.0".into(),
            patched: Vec::new(),
        };
        let summary = advisory.summary();
        assert!(!summary.contains('\u{1b}'));
        assert!(!summary.contains('\n'), "{summary}");
        assert!(
            summary.ends_with("31m title; no fixed version is listed"),
            "{summary}"
        );
    }

    #[test]
    fn the_listing_is_bounded_and_counts_the_rest() {
        let advisory = |index: usize| ReportedAdvisory {
            id: format!("RUSTSEC-2099-{index:04}"),
            title: String::new(),
            package: "p".into(),
            version: "1.0.0".into(),
            patched: vec![">=1.0.1".into()],
        };
        let many: Vec<_> = (0..LISTED_ADVISORIES + 3).map(advisory).collect();
        let lines = listing(&many, many.len());
        assert_eq!(lines.len(), LISTED_ADVISORIES + 1);
        assert_eq!(
            lines[LISTED_ADVISORIES],
            "and 3 more; run `cargo audit` for the full list"
        );
        // A count above the list is reported; an empty list prints nothing.
        let lines = listing(&many[..1], 4);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1], "and 3 more; run `cargo audit` for the full list");
        assert!(listing(&[], 2).is_empty());
        assert_eq!(listing(&many[..2], 2).len(), 2);
    }
}

//! A rule matched at several lines of a file: the JSON document keeps one
//! finding per location, and the plan and report list the lines on one
//! entry per rule and file.

use super::*;

fn rule(code: &str) -> &'static rules::Rule {
    rules::CATALOG
        .iter()
        .copied()
        .find(|rule| rule.code == code)
        .unwrap()
}

#[test]
fn the_plan_lists_every_line_of_a_rule_in_a_file_in_order() {
    let root = Path::new("/project");
    let target = Version::parse("13.0.0-alpha.6").unwrap();
    let status = rule("V13-AI-HTTP-STATUS");
    let payloads = rule("V13-ERROR-PAYLOADS");
    let at = |rule, path: &str, line| rules::SourceFinding {
        rule,
        path: root.join(path),
        line,
    };
    let scan = SourceScan {
        findings: vec![
            at(status, "src/jobs.rs", 3),
            at(status, "src/ai.rs", 40),
            at(payloads, "src/ai.rs", 20),
            at(status, "src/ai.rs", 12),
            at(status, "src/ai.rs", 77),
        ],
        ..SourceScan::default()
    };

    let text = report::plan_text(root, &target, &[], &scan);
    let entries: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("  REVIEW "))
        .collect();
    assert_eq!(entries.len(), 3, "{text}");
    assert!(
        entries[0].starts_with("  REVIEW src/ai.rs:12, 40, 77 [V13-AI-HTTP-STATUS] "),
        "{text}"
    );
    assert!(
        entries[1].starts_with("  REVIEW src/ai.rs:20 [V13-ERROR-PAYLOADS] "),
        "{text}"
    );
    assert!(
        entries[2].starts_with("  REVIEW src/jobs.rs:3 [V13-AI-HTTP-STATUS] "),
        "{text}"
    );
    assert_eq!(text.matches("migration-v13 row: ").count(), 3, "{text}");

    let markdown = report::render_report(root, &target, &[], &scan);
    let positions: Vec<usize> = [
        "- **REVIEW** `src/ai.rs` lines 12, 40, 77 (`V13-AI-HTTP-STATUS`): ",
        "- **REVIEW** `src/ai.rs` line 20 (`V13-ERROR-PAYLOADS`): ",
        "- **REVIEW** `src/jobs.rs` line 3 (`V13-AI-HTTP-STATUS`): ",
    ]
    .iter()
    .map(|entry| markdown.find(entry).expect(entry))
    .collect();
    assert!(positions.is_sorted(), "{markdown}");

    let json: serde_json::Value =
        serde_json::from_str(&report::render_json_report(root, &target, &[], &scan).unwrap())
            .unwrap();
    let lines: Vec<(&str, u64)> = json["source_findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["code"] == "V13-AI-HTTP-STATUS")
        .map(|finding| {
            (
                finding["path"].as_str().unwrap(),
                finding["line"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        lines,
        [
            ("src/jobs.rs", 3),
            ("src/ai.rs", 40),
            ("src/ai.rs", 12),
            ("src/ai.rs", 77)
        ]
    );
    assert_eq!(json["finding_counts"]["review"], 5);
}

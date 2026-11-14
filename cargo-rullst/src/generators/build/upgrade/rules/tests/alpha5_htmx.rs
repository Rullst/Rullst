//! `V13-HTMX-2`: an application created before 13.0.0-alpha.5 vendors
//! htmx 1.9.12 and not the htmx 2.0.11 new projects ship.

use super::files::Project;
use super::*;

const PACKAGE: &str = "[package]\nname = \"app\"\n";
const LEGACY: &str = "static/htmx-1.9.12.min.js";
const CURRENT: &str = "static/htmx-2.0.11.min.js";

#[test]
fn a_project_with_only_htmx_1_is_reviewed_on_that_file() {
    let project = Project::new();
    assert_eq!(project.findings(PACKAGE), vec![]);
    project.write(LEGACY, "/* htmx 1.9.12 */\n");
    assert_eq!(
        project.findings(PACKAGE),
        vec![(LEGACY.to_string(), "V13-HTMX-2", 1)]
    );
    // Moved to htmx 2, even before the old file is deleted.
    project.write(CURRENT, "/* htmx 2.0.11 */\n");
    assert_eq!(project.findings(PACKAGE), vec![]);
}

#[test]
fn a_new_project_or_a_directory_with_the_old_name_is_not_reported() {
    let new = Project::new();
    new.write(CURRENT, "/* htmx 2.0.11 */\n");
    assert_eq!(new.findings(PACKAGE), vec![]);
    let directory = Project::new();
    std::fs::create_dir_all(directory.root().join(LEGACY)).unwrap();
    assert_eq!(directory.findings(PACKAGE), vec![]);
}

#[test]
fn the_rule_is_an_alpha_5_review_with_the_move_steps() {
    let rule = CATALOG
        .iter()
        .find(|rule| rule.code == "V13-HTMX-2")
        .expect("V13-HTMX-2 in the catalog");
    assert_eq!(rule.kind, FindingKind::Review);
    assert_eq!(rule.introduced, "13.0.0-alpha.5");
    assert_eq!(rule.row, "htmx 2 client");
    for step in [
        "htmx-2.0.11.min.js",
        "htmx-ext-sse-2.2.4.js",
        "HTMX-EXT-SSE-LICENSE",
        "X-CSRF-Token",
        "hx-on:click",
    ] {
        assert!(rule.guidance.contains(step), "{step}");
    }
}

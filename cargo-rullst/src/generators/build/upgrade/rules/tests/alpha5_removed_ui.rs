//! Rules of 13.0.0-alpha.5 for the removed Wasm islands and Live components:
//! their attributes and paths in Rust source, and the removed CLI commands
//! in scripts.

use super::super::files::{runs_island_command, runs_live_command};
use super::files::Project;
use super::*;

const ISLANDS: &str = "V13-ISLANDS-REMOVED";
const LIVE: &str = "V13-LIVE-REMOVED";
const PACKAGE: &str = "[package]\nname = \"app\"\n";

fn removed_ui(source: &str) -> Vec<(&'static str, usize)> {
    scan_with(source, "lib.rs", &WorkspaceFacts::default())
        .into_iter()
        .filter(|(code, _)| *code == ISLANDS || *code == LIVE)
        .collect()
}

#[test]
fn island_attributes_and_paths_must_change() {
    let source = r#"
use rullst::island;
use rullst::{html, island as wasm_island};
#[island]
pub fn counter(start: i32) -> String { format!("{start}") }
#[rullst::island]
pub fn chart() -> String { String::new() }
#[::rullst_core::island]
fn other() {}
"#;
    assert_eq!(
        removed_ui(source),
        [
            (ISLANDS, 2),
            (ISLANDS, 3),
            (ISLANDS, 4),
            (ISLANDS, 6),
            (ISLANDS, 8)
        ]
    );
}

#[test]
fn live_components_events_and_paths_must_change() {
    let source = r#"
use rullst::live::{Live, LiveComponent};
use rullst::{html, live_component, live::live_ws_handler};
use rullst_core::live::recovery::LiveRecovery;
#[live_component]
struct Counter { count: i32 }
#[rullst::live_component]
impl Counter {
    #[rullst::live_event]
    async fn increment(&mut self) {}
}
async fn page() -> Markup {
    html! { <main>{rullst::live::Live::mount::<Counter>("/ws").await}</main> }
}
fn routes() -> Router {
    routes![get("/ws" => rullst::live::live_ws_handler::<Counter>)]
}
"#;
    assert_eq!(
        removed_ui(source),
        [
            (LIVE, 2),
            (LIVE, 3),
            (LIVE, 4),
            (LIVE, 5),
            (LIVE, 7),
            (LIVE, 9),
            (LIVE, 13),
            (LIVE, 16)
        ]
    );
}

#[test]
fn other_names_comments_strings_and_kept_apis_do_not_match() {
    let source = r##"
//! Docs may mention `#[island]` and rullst::live::Live::mount.
/// Formerly `#[live_component]`; see rullst::live.
#[doc = "rullst::live"]
#[derive(Debug)]
#[cfg_attr(test, allow(dead_code))]
struct Island { live: bool }
use rullst::realtime::{BroadcastManager, RealtimeSse};
use rullst::{html, server_function};
use my_app::live::Feed;
const TEXT: &str = "#[island] rullst::live";
fn island() -> bool { let live = true; app::live::check(live) }
fn view() -> Markup { html! { <a href="#live" data-island="x">"live"</a> } }
#[my_macros::islands]
fn kept() {}
"##;
    assert_eq!(removed_ui(source), []);
}

#[test]
fn scripts_running_the_removed_commands_are_reviewed() {
    let project = Project::new();
    project
        .write(
            ".github/workflows/ci.yml",
            "steps:\n  - run: cargo rullst build\n  - run: cargo rullst build:client --debug\n",
        )
        .write(
            "dev.sh",
            "cargo rullst make:live Counter\ncargo rullst make:island Chart\n",
        );
    let mut found = project.findings(PACKAGE);
    found.sort();
    assert_eq!(
        found,
        [
            (
                ".github/workflows/ci.yml".to_string(),
                "V13-ISLAND-COMMANDS-REMOVED",
                3
            ),
            ("dev.sh".to_string(), "V13-ISLAND-COMMANDS-REMOVED", 2),
            ("dev.sh".to_string(), "V13-LIVE-COMMAND-REMOVED", 1),
        ]
    );
    for line in [
        "cargo rullst make:island Chart",
        "rullst build:client",
        "x && cargo rullst build:client",
    ] {
        assert!(runs_island_command(line), "{line}");
    }
    for line in [
        "cargo rullst build",
        "cargo rullst make:islands",
        "rullst build:client-side",
        "echo make:live",
    ] {
        assert!(!runs_island_command(line), "{line}");
    }
    assert!(runs_live_command("cargo rullst make:live Counter"));
    assert!(!runs_live_command("cargo rullst make:lively"));
}

#[test]
fn the_alpha5_rules_name_their_rows_kinds_and_release() {
    let alpha4 = Version::parse("13.0.0-alpha.4").unwrap();
    let alpha5 = Version::parse("13.0.0-alpha.5").unwrap();
    for (rule, kind, row) in [
        (
            &catalog::ISLANDS_REMOVED,
            FindingKind::MustChange,
            "Wasm islands removed",
        ),
        (
            &catalog::ISLAND_COMMANDS_REMOVED,
            FindingKind::Review,
            "Wasm islands removed",
        ),
        (
            &catalog::LIVE_REMOVED,
            FindingKind::MustChange,
            "Live components removed",
        ),
        (
            &catalog::LIVE_COMMAND_REMOVED,
            FindingKind::Review,
            "Live components removed",
        ),
    ] {
        assert_eq!(rule.kind, kind, "{}", rule.code);
        assert_eq!(rule.row, row, "{}", rule.code);
        assert!(rule.introduced_after(&alpha4), "{}", rule.code);
        assert!(!rule.introduced_after(&alpha5), "{}", rule.code);
    }
    assert!(catalog::LIVE_REMOVED.guidance.contains("RealtimeSse"));
    assert!(catalog::ISLANDS_REMOVED.guidance.contains("htmx"));
}

//! The web starters carry the production monitor wiring commented out:
//! nothing is mounted until the application enables it and sets a token.

use super::*;

const NOTE: &str = "        // Production monitoring is off. To serve the read-only\n        // `GET /_rullst/monitor/v1` document, uncomment the next lines and set\n        // RULLST_MONITOR_TOKENS (see .env.example); without a token nothing is\n        // mounted. Guide: https://rullst.github.io/Rullst/book/production-monitoring.html\n";
const WIRING: &str = "        // .with_monitor(\n        //     rullst::monitor::Monitor::new()\n        //         .with_application(env!(\"CARGO_PKG_NAME\"), env!(\"CARGO_PKG_VERSION\")),\n        // )\n        .run(3000)\n";

/// `src/main.rs` of a starter variant.
fn main_source(manifest: Vec<(&'static str, String)>) -> String {
    manifest
        .into_iter()
        .find_map(|(path, source)| (path == "src/main.rs").then_some(source))
        .expect("src/main.rs")
}

#[test]
fn web_starters_carry_the_monitor_wiring_commented_out() {
    let (orm, frontend) = ("Active Record", "Zero-Bundle HTMX");
    let mut checked = 0;
    for hot in [false, true] {
        for (name, manifest) in [
            (
                "blank",
                blank::file_manifest("demo", "demo", false, hot, true, orm, frontend),
            ),
            (
                "blank without database",
                blank::file_manifest("demo", "demo", false, hot, false, orm, frontend),
            ),
            (
                "blank api",
                blank::file_manifest("demo", "demo", true, hot, true, orm, frontend),
            ),
            ("saas", saas::file_manifest("demo", hot, orm, frontend)),
        ] {
            let main = main_source(manifest);
            assert!(
                main.contains(&format!("{NOTE}{WIRING}")),
                "{name} (hot: {hot}):\n{main}"
            );
            assert_eq!(main.matches("with_monitor").count(), 1, "{name}");
            // Uncommented, the wiring is valid Rust in the server chain.
            let enabled = main.replace(WIRING, &WIRING.replace("        // ", "        "));
            assert!(enabled.contains("\n        .with_monitor(\n"), "{name}");
            syn::parse_file(&enabled)
                .unwrap_or_else(|error| panic!("{name} (hot: {hot}): {error}"));
            checked += 1;
        }
    }
    assert_eq!(checked, 8);
}

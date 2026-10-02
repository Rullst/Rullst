//! The home menu's command palette entry: running commands with their
//! required arguments and returning to the menu with Esc.
#![allow(clippy::panic)]

use super::super::{DashboardResult, Home, run_dashboard};
use super::FakeUi;

/// Index of the palette entry in the outside home menu.
const OUTSIDE_PALETTE: usize = 2;

#[test]
fn the_palette_runs_commands_with_their_required_arguments() {
    for (choice, selections, inputs, expected) in [
        (
            "make:model",
            vec![],
            vec!["Post"],
            vec!["make:model", "Post"],
        ),
        ("completions", vec![0], vec![], vec!["completions", "bash"]),
        (
            "generate:models",
            vec![],
            vec!["sqlite", "sqlite://app.db"],
            vec![
                "generate:models",
                "--driver",
                "sqlite",
                "--url",
                "sqlite://app.db",
            ],
        ),
        ("update check", vec![], vec![], vec!["update", "check"]),
        ("doctor", vec![], vec![], vec!["doctor"]),
    ] {
        let mut ui = FakeUi {
            selections: std::iter::once(OUTSIDE_PALETTE).chain(selections).collect(),
            inputs: inputs.into_iter().map(str::to_string).collect(),
            palettes: [Some(choice.to_string())].into(),
            ..FakeUi::default()
        };
        let mut commands = Vec::new();
        run_dashboard(&mut ui, "cargo-rullst", &Home::Outside, &mut |command| {
            commands.push(command);
            Ok(())
        })
        .expect("palette choice runs");
        let mut full = vec!["cargo-rullst".to_string()];
        full.extend(expected.iter().map(|word| word.to_string()));
        assert_eq!(commands, vec![full.clone()], "{choice}");
        crate::command()
            .try_get_matches_from(&full)
            .unwrap_or_else(|error| panic!("{choice}: {error}"));
    }
}

#[test]
fn escape_in_the_palette_returns_to_the_same_home_menu() {
    let mut ui = FakeUi {
        selections: [OUTSIDE_PALETTE, 4].into(),
        palettes: [None].into(),
        ..FakeUi::default()
    };
    let mut run = |_| -> DashboardResult<()> { panic!("escape then exit must not execute") };
    run_dashboard(&mut ui, "cargo-rullst", &Home::Outside, &mut run).expect("escape returns home");
    assert_eq!(ui.brand_count, 1, "the home is drawn again");
    assert!(ui.prompts[1].starts_with("palette|"));
    assert!(ui.prompts[2].contains("Search All Commands"));
}

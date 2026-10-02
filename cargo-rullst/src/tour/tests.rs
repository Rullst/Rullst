use super::*;
use std::collections::VecDeque;

#[derive(Default)]
struct FakeTour {
    answers: VecDeque<Answer>,
    screens: Vec<Screen>,
    printed: Vec<String>,
    missing: Vec<&'static str>,
    runs: Vec<Vec<String>>,
}

impl TourUi for FakeTour {
    fn choose(&mut self, screen: &Screen) -> io::Result<Answer> {
        self.screens.push(screen.clone());
        self.answers
            .pop_front()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "no scripted answer"))
    }

    fn print(&mut self, lines: &[Line]) -> io::Result<()> {
        self.printed.extend(lines.iter().map(Line::text));
        Ok(())
    }

    fn available(&mut self, arguments: &[&str]) -> bool {
        arguments
            .first()
            .is_some_and(|subcommand| !self.missing.contains(subcommand))
    }

    fn run(&mut self, arguments: &[&str]) -> io::Result<Option<i32>> {
        self.runs
            .push(arguments.iter().map(|word| word.to_string()).collect());
        Ok(Some(0))
    }
}

fn tour(answers: &[Answer]) -> FakeTour {
    FakeTour {
        answers: answers.iter().cloned().collect(),
        ..FakeTour::default()
    }
}

#[test]
fn the_tour_covers_the_main_commands_in_five_to_seven_steps() {
    let commands: Vec<&str> = STEPS.iter().map(|step| step.command).collect();
    assert_eq!(
        commands,
        ["new", "dev", "dash", "make:*", "db:*", "doctor", "ai"]
    );
    for step in STEPS {
        assert!(!step.explanation.is_empty() && !step.try_commands.is_empty());
        for command in step.try_commands {
            assert!(command.starts_with("cargo rullst "), "{command}");
        }
        let arguments = step.example.arguments;
        let read_only = arguments.contains(&"--help") || arguments.contains(&"--dry-run");
        assert!(read_only, "{} example must be read-only", step.command);
        if let Some(fallback) = step.example.fallback {
            assert!(fallback.contains(&"--help"));
        }
    }
}

#[test]
fn list_prints_every_step_without_prompting_or_colour_codes() {
    let text: Vec<String> = list_lines().iter().map(Line::text).collect();
    assert_eq!(text[0], "Rullst tour · 7 steps");
    assert!(text.contains(&"1. Create a project  (new)".to_string()));
    assert!(text.contains(&"7. Work with AI  (ai)".to_string()));
    assert!(text.contains(
        &"   Example  cargo rullst new tour-demo --default --dry-run  (previews a project; creates nothing)"
            .to_string()
    ));
    assert!(text.contains(&"   Try      cargo rullst make:model Post -m".to_string()));
    assert!(text.iter().all(|line| !line.contains('\x1b')));
}

#[test]
fn walking_through_runs_nothing_and_ends_with_the_first_command() {
    let answers: Vec<Answer> = (0..STEPS.len()).map(|_| Answer::One(0)).collect();
    let mut ui = tour(&answers);
    guided(&mut ui).unwrap();
    assert!(ui.runs.is_empty(), "no example ran without being picked");
    assert_eq!(ui.screens.len(), 7);
    assert_eq!(ui.screens[0].crumb, "Step 1 of 7 · new");
    assert!(!ui.screens[0].can_go_back);
    assert_eq!(ui.screens[6].choices[0].label, "Finish");
    assert_eq!(
        ui.printed.last().map(String::as_str),
        Some("◆ That's the tour. Create your first app: cargo rullst new my_app")
    );
}

#[test]
fn examples_run_only_when_picked_and_the_step_is_shown_again() {
    let mut ui = tour(&[
        Answer::One(1),
        Answer::One(0),
        Answer::One(1),
        Answer::One(3),
    ]);
    guided(&mut ui).unwrap();
    assert_eq!(
        ui.runs,
        [
            vec!["new", "tour-demo", "--default", "--dry-run"],
            vec!["dev", "--help"],
        ]
    );
    assert_eq!(ui.screens[0].crumb, ui.screens[1].crumb);
    assert!(ui.printed.contains(
        &"↳ `cargo rullst new tour-demo --default --dry-run` finished (exit status 0)".to_string()
    ));
    assert_eq!(
        ui.printed.last().map(String::as_str),
        Some("Tour closed. `cargo rullst tour --list` prints every step.")
    );
}

#[test]
fn back_and_previous_return_and_quit_stops_immediately() {
    let mut ui = tour(&[
        Answer::Back,
        Answer::One(0),
        Answer::Back,
        Answer::One(0),
        Answer::One(2),
        Answer::One(2),
    ]);
    guided(&mut ui).unwrap();
    let crumbs: Vec<&str> = ui
        .screens
        .iter()
        .map(|screen| screen.crumb.as_str())
        .collect();
    assert_eq!(
        crumbs,
        [
            "Step 1 of 7 · new",
            "Step 1 of 7 · new",
            "Step 2 of 7 · dev",
            "Step 1 of 7 · new",
            "Step 2 of 7 · dev",
            "Step 1 of 7 · new",
        ]
    );
    assert!(ui.runs.is_empty());
}

#[test]
fn a_missing_ai_command_falls_back_to_the_context_generator() {
    let mut answers = vec![Answer::One(0); 6];
    answers.extend([Answer::One(1), Answer::One(0)]);
    let mut ui = tour(&answers);
    ui.missing = vec!["ai"];
    guided(&mut ui).unwrap();
    assert_eq!(ui.runs, [vec!["generate:ai-context", "--help"]]);
    assert!(ui.printed.contains(
        &"`cargo rullst ai` is not part of this build; showing `cargo rullst generate:ai-context --help` instead."
            .to_string()
    ));

    let mut ui = tour(&[Answer::One(1), Answer::One(2)]);
    ui.missing = vec!["new"];
    guided(&mut ui).unwrap();
    assert!(ui.runs.is_empty());
    assert!(
        ui.printed
            .contains(&"`cargo rullst new` is not part of this build.".to_string())
    );
}

#[test]
fn the_tour_command_offers_a_non_interactive_list() {
    let matches = command()
        .try_get_matches_from(["tour", "--list"])
        .expect("--list parses");
    assert!(matches.get_flag("list"));
    assert!(command().try_get_matches_from(["tour", "--bogus"]).is_err());
}

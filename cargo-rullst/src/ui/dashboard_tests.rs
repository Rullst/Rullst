#![allow(clippy::expect_used, clippy::panic)]

use std::{collections::VecDeque, io};

use super::{
    DashboardResult, DashboardUi, handle_auth_billing, handle_database_operations, handle_deploy,
    handle_existing_project, handle_scaffold_code, run_dashboard,
};

#[derive(Default)]
struct FakeUi {
    selections: VecDeque<usize>,
    inputs: VecDeque<String>,
    prompts: Vec<String>,
    brand_count: usize,
}

impl FakeUi {
    fn with_selections(selections: impl IntoIterator<Item = usize>) -> Self {
        Self {
            selections: selections.into_iter().collect(),
            ..Self::default()
        }
    }

    fn with_input(selection: usize, input: &str) -> Self {
        Self {
            selections: [selection].into(),
            inputs: [input.to_string()].into(),
            ..Self::default()
        }
    }
}

impl DashboardUi for FakeUi {
    fn show_brand(&mut self) -> DashboardResult<()> {
        self.brand_count = self.brand_count.saturating_add(1);
        Ok(())
    }

    fn select(&mut self, prompt: &str, choices: &[String]) -> DashboardResult<usize> {
        self.prompts.push(format!("{prompt}|{}", choices.join("|")));
        self.selections.pop_front().ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "missing fake selection").into()
        })
    }

    fn input(&mut self, prompt: &str) -> DashboardResult<String> {
        self.prompts.push(prompt.to_string());
        self.inputs.pop_front().ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "missing fake input").into()
        })
    }
}

#[test]
fn dashboard_never_indexes_an_empty_command() {
    assert!(super::execute_command(Vec::new()).is_err());
}

#[test]
fn scaffold_menu_maps_every_choice_to_the_documented_command() {
    let named_cases = [
        (0, "make:controller", Vec::<&str>::new()),
        (1, "make:model", vec!["-m"]),
        (2, "make:middleware", vec![]),
        (3, "make:worker", vec![]),
        (4, "make:migration", vec![]),
        (5, "make:live", vec![]),
        (6, "make:island", vec![]),
    ];
    for (selection, action, extra) in named_cases {
        let mut ui = FakeUi::with_input(selection, "Example");
        let mut commands = Vec::new();
        handle_scaffold_code(&mut ui, "cargo-rullst", &mut |command| {
            commands.push(command);
            Ok(())
        })
        .expect("named scaffold choice should be accepted");

        let mut expected = vec![
            "cargo-rullst".to_string(),
            action.to_string(),
            "Example".to_string(),
        ];
        expected.extend(extra.into_iter().map(str::to_string));
        assert_eq!(commands, vec![expected]);
        assert_eq!(ui.prompts.len(), 2);
    }

    for (selection, action) in [
        (7, "make:scalar"),
        (8, "make:k8s"),
        (9, "make:grpc"),
        (10, "generate:models"),
    ] {
        let mut ui = FakeUi::with_selections([selection]);
        let mut commands = Vec::new();
        handle_scaffold_code(&mut ui, "cargo-rullst", &mut |command| {
            commands.push(command);
            Ok(())
        })
        .expect("direct scaffold choice should be accepted");
        assert_eq!(
            commands,
            vec![vec!["cargo-rullst".to_string(), action.to_string()]]
        );
    }

    let mut ui = FakeUi::with_selections([usize::MAX]);
    let mut run = |_| -> DashboardResult<()> { panic!("invalid selection must not execute") };
    handle_scaffold_code(&mut ui, "cargo-rullst", &mut run)
        .expect("unknown scaffold choices are ignored");
}

#[test]
fn database_auth_and_deploy_menus_map_every_choice() {
    for (selection, action) in [
        (0, "db:migrate"),
        (1, "db:rollback"),
        (2, "db:status"),
        (3, "db:seed"),
        (4, "studio"),
    ] {
        let mut ui = FakeUi::with_selections([selection]);
        let mut commands = Vec::new();
        handle_database_operations(&mut ui, "cargo-rullst", &mut |command| {
            commands.push(command);
            Ok(())
        })
        .expect("database choice should be accepted");
        assert_eq!(
            commands,
            vec![vec!["cargo-rullst".to_string(), action.to_string()]]
        );
    }

    for (selection, expected) in [
        (0, vec!["cargo-rullst", "auth"]),
        (1, vec!["cargo-rullst", "make:mfa"]),
        (
            2,
            vec![
                "cargo-rullst",
                "audit",
                "--ai",
                "--compliance",
                "--idor",
                "--geiger",
            ],
        ),
        (3, vec!["cargo-rullst", "make:billing"]),
        (4, vec!["cargo-rullst", "make:cors"]),
        (5, vec!["cargo-rullst", "make:jwt"]),
    ] {
        let mut ui = FakeUi::with_selections([selection]);
        let mut commands = Vec::new();
        handle_auth_billing(&mut ui, "cargo-rullst", &mut |command| {
            commands.push(command);
            Ok(())
        })
        .expect("auth choice should be accepted");
        assert_eq!(
            commands,
            vec![expected.into_iter().map(str::to_string).collect::<Vec<_>>()]
        );
    }

    for (selection, action) in [(0, "deploy"), (1, "foundry:init"), (2, "foundry:deploy")] {
        let mut ui = FakeUi::with_selections([selection]);
        let mut commands = Vec::new();
        handle_deploy(&mut ui, "cargo-rullst", &mut |command| {
            commands.push(command);
            Ok(())
        })
        .expect("deploy choice should be accepted");
        assert_eq!(
            commands,
            vec![vec!["cargo-rullst".to_string(), action.to_string()]]
        );
    }

    for handler in [
        handle_database_operations::<FakeUi, fn(Vec<String>) -> DashboardResult<()>>,
        handle_auth_billing::<FakeUi, fn(Vec<String>) -> DashboardResult<()>>,
        handle_deploy::<FakeUi, fn(Vec<String>) -> DashboardResult<()>>,
    ] {
        let mut ui = FakeUi::with_selections([usize::MAX]);
        let mut run: fn(Vec<String>) -> DashboardResult<()> =
            |_| panic!("invalid selection must not execute");
        handler(&mut ui, "cargo-rullst", &mut run).expect("unknown submenu choices are ignored");
    }
}

#[test]
fn project_menu_reaches_direct_nested_and_back_paths() {
    for (selection, action) in [
        (0, "dev"),
        (1, "dash"),
        (5, "make:omni"),
        (6, "dockerize"),
        (7, "nixify"),
        (9, "upgrade"),
    ] {
        let mut ui = FakeUi::with_selections([selection]);
        let mut commands = Vec::new();
        handle_existing_project(&mut ui, "cargo-rullst", &mut |command| {
            commands.push(command);
            Ok(())
        })
        .expect("project choice should be accepted");
        assert_eq!(
            commands,
            vec![vec!["cargo-rullst".to_string(), action.to_string()]]
        );
    }

    for (selections, action) in [
        (vec![2, 7], "make:scalar"),
        (vec![3, 2], "db:status"),
        (vec![4, 1], "make:mfa"),
        (vec![8, 2], "foundry:deploy"),
    ] {
        let mut ui = FakeUi::with_selections(selections);
        let mut commands = Vec::new();
        handle_existing_project(&mut ui, "cargo-rullst", &mut |command| {
            commands.push(command);
            Ok(())
        })
        .expect("nested project choice should be accepted");
        assert_eq!(
            commands,
            vec![vec!["cargo-rullst".to_string(), action.to_string()]]
        );
    }

    let mut ui = FakeUi::with_selections([10, 3]);
    let mut run = |_| -> DashboardResult<()> { panic!("back then exit must not execute") };
    handle_existing_project(&mut ui, "cargo-rullst", &mut run)
        .expect("back should return to the main dashboard");
    assert_eq!(ui.prompts.len(), 2);
    assert_eq!(ui.brand_count, 1);

    let mut ui = FakeUi::with_selections([usize::MAX]);
    handle_existing_project(&mut ui, "cargo-rullst", &mut run)
        .expect("unknown project choices are ignored");
}

#[test]
fn main_menu_reaches_new_existing_help_exit_and_unknown_paths() {
    let mut ui = FakeUi::with_selections([0]);
    let mut commands = Vec::new();
    run_dashboard(&mut ui, "cargo-rullst", &mut |command| {
        commands.push(command);
        Ok(())
    })
    .expect("new-project choice should be accepted");
    assert_eq!(
        commands,
        vec![vec!["cargo-rullst".to_string(), "new".to_string()]]
    );

    let mut ui = FakeUi::with_selections([1, 5]);
    let mut commands = Vec::new();
    run_dashboard(&mut ui, "cargo-rullst", &mut |command| {
        commands.push(command);
        Ok(())
    })
    .expect("existing-project choice should be accepted");
    assert_eq!(
        commands,
        vec![vec!["cargo-rullst".to_string(), "make:omni".to_string()]]
    );

    for selection in [2, 3, usize::MAX] {
        let mut ui = FakeUi::with_selections([selection]);
        let mut run = |_| -> DashboardResult<()> { panic!("choice must not execute") };
        run_dashboard(&mut ui, "cargo-rullst", &mut run)
            .expect("non-command dashboard choice should be accepted");
        assert_eq!(ui.prompts.len(), 1);
    }
}

#[test]
fn interaction_input_and_runner_errors_propagate() {
    let mut missing_selection = FakeUi::default();
    let mut run = |_| Ok(());
    assert!(run_dashboard(&mut missing_selection, "cargo-rullst", &mut run).is_err());

    let mut missing_input = FakeUi::with_selections([0]);
    assert!(handle_scaffold_code(&mut missing_input, "cargo-rullst", &mut run).is_err());

    let mut ui = FakeUi::with_selections([0]);
    let mut failing_runner =
        |_| -> DashboardResult<()> { Err(io::Error::other("simulated runner failure").into()) };
    assert!(run_dashboard(&mut ui, "cargo-rullst", &mut failing_runner).is_err());
}

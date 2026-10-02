#![allow(clippy::expect_used, clippy::panic)]

use std::{collections::VecDeque, io};

use super::{
    DashboardResult, DashboardUi, Home, PaletteEntry, handle_auth_billing,
    handle_database_operations, handle_deploy, handle_existing_project, handle_scaffold_code,
    run_dashboard,
};

#[derive(Default)]
struct FakeUi {
    selections: VecDeque<usize>,
    inputs: VecDeque<String>,
    palettes: VecDeque<Option<String>>,
    prompts: Vec<String>,
    brand_count: usize,
    geiger: bool,
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
    fn show_home(&mut self, _home: &Home) -> DashboardResult<()> {
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

    fn palette(&mut self, entries: &[PaletteEntry]) -> DashboardResult<Option<usize>> {
        self.prompts.push(format!("palette|{}", entries.len()));
        let choice = self.palettes.pop_front().ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "missing fake palette choice")
        })?;
        Ok(choice.and_then(|name| entries.iter().position(|entry| entry.name == name)))
    }

    fn geiger_available(&mut self) -> bool {
        self.geiger
    }
}

#[path = "dashboard_palette_tests.rs"]
mod palette;

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
        (9, "make:grpc", vec![]),
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

    for (selection, action) in [(7, "make:scalar"), (8, "make:k8s")] {
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
fn model_introspection_collects_the_required_driver_and_url() {
    let mut ui = FakeUi {
        selections: [10, 1].into(),
        inputs: ["postgres://localhost/app".to_string()].into(),
        ..FakeUi::default()
    };
    let mut commands = Vec::new();
    handle_scaffold_code(&mut ui, "cargo-rullst", &mut |command| {
        commands.push(command);
        Ok(())
    })
    .expect("model introspection choice should be accepted");
    assert_eq!(
        commands,
        vec![
            [
                "cargo-rullst",
                "generate:models",
                "--driver",
                "postgres",
                "--url",
                "postgres://localhost/app",
            ]
            .map(str::to_string)
            .to_vec()
        ]
    );
    // Every argument clap requires is present.
    let parsed = <crate::cli::Cli as clap::Parser>::try_parse_from(&commands[0])
        .expect("generate:models arguments must parse");
    assert!(matches!(
        parsed.command,
        crate::cli::Commands::GenerateModels { .. }
    ));

    let mut ui = FakeUi::with_selections([10, usize::MAX]);
    let mut run = |_| -> DashboardResult<()> { panic!("an unknown driver must not execute") };
    handle_scaffold_code(&mut ui, "cargo-rullst", &mut run).expect("unknown drivers are ignored");
}

#[test]
fn grpc_scaffold_passes_the_required_service_name() {
    let mut ui = FakeUi::with_input(9, "UserService");
    let mut commands = Vec::new();
    handle_scaffold_code(&mut ui, "cargo-rullst", &mut |command| {
        commands.push(command);
        Ok(())
    })
    .expect("gRPC choice should be accepted");
    let parsed = <crate::cli::Cli as clap::Parser>::try_parse_from(&commands[0])
        .expect("make:grpc arguments must parse");
    assert!(matches!(
        parsed.command,
        crate::cli::Commands::MakeGrpc { ref name } if name == "UserService"
    ));
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
        // Without cargo-geiger, requesting it would fail the whole audit.
        (
            2,
            vec!["cargo-rullst", "audit", "--ai", "--compliance", "--idor"],
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

    let mut ui = FakeUi {
        geiger: true,
        ..FakeUi::with_selections([2])
    };
    let mut commands = Vec::new();
    handle_auth_billing(&mut ui, "cargo-rullst", &mut |command| {
        commands.push(command);
        Ok(())
    })
    .expect("audit choice should be accepted");
    assert_eq!(commands[0].last().map(String::as_str), Some("--geiger"));

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
        handle_existing_project(&mut ui, "cargo-rullst", &Home::Outside, &mut |command| {
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
        handle_existing_project(&mut ui, "cargo-rullst", &Home::Outside, &mut |command| {
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
    handle_existing_project(&mut ui, "cargo-rullst", &Home::Outside, &mut run)
        .expect("back should return to the main dashboard");
    assert_eq!(ui.prompts.len(), 2);
    assert_eq!(ui.brand_count, 1);

    let mut ui = FakeUi::with_selections([usize::MAX]);
    handle_existing_project(&mut ui, "cargo-rullst", &Home::Outside, &mut run)
        .expect("unknown project choices are ignored");
}

#[test]
fn main_menu_reaches_new_existing_help_exit_and_unknown_paths() {
    let mut ui = FakeUi::with_selections([0]);
    let mut commands = Vec::new();
    run_dashboard(&mut ui, "cargo-rullst", &Home::Outside, &mut |command| {
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
    run_dashboard(&mut ui, "cargo-rullst", &Home::Outside, &mut |command| {
        commands.push(command);
        Ok(())
    })
    .expect("existing-project choice should be accepted");
    assert_eq!(
        commands,
        vec![vec!["cargo-rullst".to_string(), "make:omni".to_string()]]
    );

    for selection in [3, 4, usize::MAX] {
        let mut ui = FakeUi::with_selections([selection]);
        let mut run = |_| -> DashboardResult<()> { panic!("choice must not execute") };
        run_dashboard(&mut ui, "cargo-rullst", &Home::Outside, &mut run)
            .expect("non-command dashboard choice should be accepted");
        assert_eq!(ui.prompts.len(), 1);
    }
}

#[test]
fn interaction_input_and_runner_errors_propagate() {
    let mut missing_selection = FakeUi::default();
    let mut run = |_| Ok(());
    assert!(
        run_dashboard(
            &mut missing_selection,
            "cargo-rullst",
            &Home::Outside,
            &mut run
        )
        .is_err()
    );

    let mut missing_input = FakeUi::with_selections([0]);
    assert!(handle_scaffold_code(&mut missing_input, "cargo-rullst", &mut run).is_err());

    let mut ui = FakeUi::with_selections([0]);
    let mut failing_runner =
        |_| -> DashboardResult<()> { Err(io::Error::other("simulated runner failure").into()) };
    assert!(run_dashboard(&mut ui, "cargo-rullst", &Home::Outside, &mut failing_runner).is_err());
}

fn project_home(relative_root: Option<&str>) -> Home {
    Home::Project(Box::new(crate::ui::home::Project {
        root: std::path::PathBuf::from("/work/shop"),
        relative_root: relative_root.map(str::to_string),
        name: "shop".to_string(),
        features: Vec::new(),
        database: crate::ui::home::Database::NotConfigured,
        migrations: None,
        git_branch: None,
    }))
}

#[test]
fn project_home_offers_quick_actions_and_every_submenu() {
    let home = project_home(None);
    for (selections, expected) in [
        (vec![0], "dev"),
        (vec![1], "dash"),
        (vec![2, 7], "make:scalar"),
        (vec![3, 0], "db:migrate"),
        (vec![4], "doctor"),
        (vec![5, 0], "deploy"),
        (vec![6, 6], "dockerize"),
        (vec![7], "new"),
    ] {
        let mut ui = FakeUi::with_selections(selections.clone());
        let mut commands = Vec::new();
        run_dashboard(&mut ui, "cargo-rullst", &home, &mut |command| {
            commands.push(command);
            Ok(())
        })
        .expect("project home choice should be accepted");
        assert_eq!(
            commands,
            vec![vec!["cargo-rullst".to_string(), expected.to_string()]],
            "{selections:?}"
        );
    }

    for selection in [9, 10, usize::MAX] {
        let mut ui = FakeUi::with_selections([selection]);
        let mut run = |_| -> DashboardResult<()> { panic!("choice must not execute") };
        run_dashboard(&mut ui, "cargo-rullst", &home, &mut run)
            .expect("non-command project home choice should be accepted");
    }

    // Back from all project operations returns to the same project home.
    let mut ui = FakeUi::with_selections([6, 10, 10]);
    let mut run = |_| -> DashboardResult<()> { panic!("back then exit must not execute") };
    run_dashboard(&mut ui, "cargo-rullst", &home, &mut run).expect("back returns home");
    assert_eq!(ui.brand_count, 1);
    assert_eq!(ui.prompts.len(), 3);
    assert!(ui.prompts[2].contains("Start Dev Server"));
}

#[test]
fn the_outside_home_leads_with_project_creation() {
    let mut ui = FakeUi::with_selections([4]);
    let mut run = |_| -> DashboardResult<()> { panic!("exit must not execute") };
    run_dashboard(&mut ui, "cargo-rullst", &Home::Outside, &mut run).expect("exit");
    let menu = &ui.prompts[0];
    assert!(menu.contains("Create New Project"));
    assert!(menu.find("Create New Project") < menu.find("Already have a project?"));
}

#[test]
fn relative_programs_are_anchored_before_running_at_the_project_root() {
    let current = std::path::Path::new("/work/shop/src");
    assert_eq!(
        super::resolve_program("./target/debug/cargo-rullst", current),
        current
            .join("./target/debug/cargo-rullst")
            .display()
            .to_string()
    );
    assert_eq!(
        super::resolve_program("cargo-rullst", current),
        "cargo-rullst"
    );
    #[cfg(unix)]
    assert_eq!(
        super::resolve_program("/usr/bin/cargo-rullst", current),
        "/usr/bin/cargo-rullst"
    );
}

#[cfg(unix)]
#[test]
fn menu_commands_can_run_from_the_project_root() {
    let root = tempfile::tempdir().expect("temporary project root");
    std::fs::write(root.path().join("Cargo.toml"), "").expect("marker");
    let check = ["sh", "-c", "test -f Cargo.toml"]
        .map(str::to_string)
        .to_vec();
    super::execute_command_in(Some(root.path()), check.clone()).expect("runs at the root");
    let elsewhere = tempfile::tempdir().expect("another directory");
    assert!(super::execute_command_in(Some(elsewhere.path()), check).is_err());
}

#[cfg(unix)]
#[test]
fn a_failed_menu_command_keeps_its_own_report_and_exit_status() {
    let failing = ["sh", "-c", "exit 2"].map(str::to_string).to_vec();
    let error = super::execute_command_in(None, failing).expect_err("exit status 2");
    // The child already printed its report: the parent adds none and keeps
    // the usage status instead of a generic "Command failed" with exit 1.
    assert_eq!(super::super::error_report::report(error.as_ref(), &[]), 2);
    assert!(error.to_string().contains("failed with status"));

    let crashed = ["sh", "-c", "kill -9 $$"].map(str::to_string).to_vec();
    let error = super::execute_command_in(None, crashed).expect_err("killed by a signal");
    assert!(
        error
            .downcast_ref::<super::super::error_report::AlreadyReported>()
            .is_none()
    );
}

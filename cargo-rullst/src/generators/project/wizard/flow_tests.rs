use super::super::{NewProjectRequest, initial_plan, interactive_setup, requested_integrations};
use super::*;
use crate::blueprints::{LMS_BLUEPRINT_ID, SAAS_BLUEPRINT_ID};
use crate::generators::project::ProjectScaffoldOptions;
use std::collections::VecDeque;
use std::io;

#[derive(Default)]
struct FakeUi {
    names: VecDeque<&'static str>,
    answers: VecDeque<Answer>,
    rejected: Vec<String>,
    screens: Vec<Screen>,
}

impl WizardUi for FakeUi {
    fn ask_name(&mut self, _initial: &str) -> WizardResult<String> {
        self.names
            .pop_front()
            .map(str::to_string)
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "no name").into())
    }

    fn reject_name(&mut self, reason: &str) {
        self.rejected.push(reason.to_string());
    }

    fn choose(&mut self, screen: &Screen) -> WizardResult<Answer> {
        self.screens.push(screen.clone());
        self.answers
            .pop_front()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "no answer").into())
    }
}

impl FakeUi {
    fn scripted(names: &[&'static str], answers: Vec<Answer>) -> Self {
        Self {
            names: names.iter().copied().collect(),
            answers: answers.into(),
            ..Self::default()
        }
    }

    fn crumbs(&self) -> Vec<String> {
        self.screens
            .iter()
            .map(|screen| screen.crumb.clone())
            .collect()
    }
}

fn setup(request: NewProjectRequest<'_>) -> Setup {
    let requested = requested_integrations(&request.options);
    let plan = initial_plan(&request, &requested);
    interactive_setup(&request, plan, true, 3000)
}

/// A preview that names the blueprint, so tests can see which plan it got.
fn fake_preview(plan: &ProjectPlan) -> Option<Vec<String>> {
    let flag = super::super::catalog::blueprint(plan.blueprint)?.flag;
    Some(vec![
        "Cargo.toml".to_string(),
        format!("src/{flag}.rs"),
        "src/models/a.rs".to_string(),
        "src/models/b.rs".to_string(),
    ])
}

fn run_with(request: NewProjectRequest<'_>, ui: &mut FakeUi) -> WizardResult<Outcome> {
    run(setup(request), ui, fake_preview)
}

fn created(outcome: WizardResult<Outcome>) -> ProjectPlan {
    match outcome.expect("wizard result") {
        Outcome::Create(plan) => plan,
        Outcome::Cancelled => panic!("wizard was cancelled"),
    }
}

fn texts(lines: &[Line]) -> Vec<String> {
    lines.iter().map(Line::text).collect()
}

#[test]
fn interactive_blank_wizard_validates_names_and_composes_the_full_profile() {
    let mut ui = FakeUi::scripted(
        &["", "space name", "1number", "bad!", "crate", "learning_hub"],
        vec![
            Answer::One(0),                    // Blank
            Answer::One(1),                    // JSON API
            Answer::One(4),                    // Turso primary
            Answer::Many(vec![0, 4, 5, 6, 7]), // AI + every add-on offered
            Answer::One(0),                    // Create
        ],
    );
    let plan = created(run_with(NewProjectRequest::default(), &mut ui));

    assert_eq!(
        ui.rejected,
        [
            "Enter a project name.",
            "Spaces are not allowed in the project name.",
            "The project name cannot start with a number.",
            "Only letters, numbers, underscores and dashes are allowed.",
            "That name is reserved by Rust; choose another one.",
        ]
    );
    let options = plan.wizard_options();
    assert_eq!(options.name, "learning_hub");
    assert!(options.api && options.db_needed && options.wants_ai && !options.wants_redis);
    assert_eq!(options.db_provider, "Turso");
    assert_eq!(options.orm_pattern, "Turso Active Record");
    assert_eq!(
        options.polyglot_integrations,
        [
            PolyglotIntegration::Turso,
            PolyglotIntegration::MongoDb,
            PolyglotIntegration::DuckDb,
            PolyglotIntegration::SurrealDb,
            PolyglotIntegration::Qdrant,
        ]
    );

    let features = &ui.screens[3];
    let labels: Vec<&str> = features
        .choices
        .iter()
        .map(|choice| choice.label.as_str())
        .collect();
    assert!(
        !labels.contains(&"Turso / libSQL add-on"),
        "the primary is not an add-on"
    );
    assert_eq!(
        features.selection,
        Selection::Many {
            checked: vec![false; 8]
        }
    );
    assert_eq!(
        ui.crumbs(),
        [
            "Step 2 of 6 · Blueprint",
            "Step 3 of 6 · Application",
            "Step 4 of 6 · Database",
            "Step 5 of 6 · Features",
            "Step 6 of 6 · Review",
        ]
    );
}

#[test]
fn blueprint_choices_describe_themselves_and_preview_their_own_tree() {
    let mut ui = FakeUi::scripted(
        &[],
        vec![
            Answer::One(2),
            Answer::One(0),
            Answer::Many(vec![]),
            Answer::One(0),
        ],
    );
    let plan = created(run_with(
        NewProjectRequest {
            name: Some("shop"),
            ..NewProjectRequest::default()
        },
        &mut ui,
    ));
    assert_eq!(plan.blueprint, SAAS_BLUEPRINT_ID);

    let blueprints = &ui.screens[0];
    assert_eq!(blueprints.choices.len(), 6);
    for choice in &blueprints.choices {
        assert!(
            !choice.hint.is_empty(),
            "{} has a description",
            choice.label
        );
    }
    let saas = texts(&blueprints.choices[2].detail);
    assert_eq!(saas[0], "shop/  4 files");
    assert!(
        saas.iter().any(|line| line.contains("saas.rs")),
        "{saas:#?}"
    );
    assert!(
        texts(&blueprints.choices[0].detail)
            .iter()
            .any(|line| line.contains("blank.rs"))
    );

    let review = ui.screens.last().expect("review");
    assert_eq!(review.choices[0].label, "Create the project");
    let body = texts(&review.body).join("\n");
    assert!(body.contains("Blueprint    SaaS Starter"), "{body}");
    assert!(body.contains("cargo run -q -- db:migrate"), "{body}");
    assert!(body.contains("cd shop && cargo rullst dev"), "{body}");
    assert!(
        texts(&review.choices[0].detail)
            .iter()
            .any(|line| line.contains("saas.rs"))
    );
}

#[test]
fn an_explicit_api_flag_skips_the_blueprint_and_build_type_questions() {
    let mut ui = FakeUi::scripted(
        &[],
        vec![Answer::One(0), Answer::Many(vec![]), Answer::One(0)],
    );
    let plan = created(run_with(
        NewProjectRequest {
            name: Some("billing-api"),
            options: ProjectScaffoldOptions {
                api: true,
                ..ProjectScaffoldOptions::default()
            },
            ..NewProjectRequest::default()
        },
        &mut ui,
    ));
    assert!(plan.api);
    assert_eq!(plan.blueprint, BLANK_BLUEPRINT_ID);
    assert_eq!(plan.database, Database::Provider("Sqlite"));
    assert_eq!(
        ui.crumbs(),
        [
            "Step 1 of 3 · Database",
            "Step 2 of 3 · Features",
            "Step 3 of 3 · Review"
        ]
    );
}

#[test]
fn profile_flags_skip_their_questions_and_their_features() {
    let mut ui = FakeUi::scripted(&[], vec![Answer::Many(vec![]), Answer::One(0)]);
    let plan = created(run_with(
        NewProjectRequest {
            name: Some("shop"),
            blueprint: Some(SAAS_BLUEPRINT_ID),
            options: ProjectScaffoldOptions {
                database: Some("Postgres"),
                wants_ai: true,
                mongodb: true,
                docker: true,
                ..ProjectScaffoldOptions::default()
            },
            ..NewProjectRequest::default()
        },
        &mut ui,
    ));
    assert_eq!(
        ui.crumbs(),
        ["Step 1 of 2 · Features", "Step 2 of 2 · Review"]
    );
    let offered: Vec<&str> = ui.screens[0]
        .choices
        .iter()
        .map(|choice| choice.label.as_str())
        .collect();
    assert_eq!(
        offered,
        [
            "Redis adapters",
            "Nix flake",
            "Turso / libSQL add-on",
            "DuckDB",
            "SurrealDB",
            "Qdrant"
        ]
    );
    assert!(plan.ai && plan.docker);
    assert_eq!(plan.database, Database::Provider("Postgres"));
    assert_eq!(plan.integrations(), [PolyglotIntegration::MongoDb]);
    let review = texts(&ui.screens[1].body).join("\n");
    assert!(
        review.contains("Features     AI features, Dockerfile, MongoDB"),
        "{review}"
    );
}

#[test]
fn back_returns_to_the_previous_question_with_the_earlier_answer_selected() {
    let mut ui = FakeUi::scripted(
        &[],
        vec![
            Answer::One(1),        // LMS
            Answer::Back,          // from Database back to Blueprint
            Answer::One(0),        // Blank
            Answer::One(0),        // Full-stack
            Answer::One(5),        // No database
            Answer::Many(vec![1]), // Redis
            Answer::One(1),        // review: Back to Features
            Answer::Many(vec![]),  // clear Redis
            Answer::One(0),        // Create
        ],
    );
    let plan = created(run_with(
        NewProjectRequest {
            name: Some("academy"),
            ..NewProjectRequest::default()
        },
        &mut ui,
    ));
    let crumbs = ui.crumbs();
    assert_eq!(crumbs[0], "Step 1 of 5 · Blueprint");
    assert_eq!(
        crumbs[1], "Step 2 of 4 · Database",
        "LMS has no build-type question"
    );
    assert_eq!(
        ui.screens[2].selection,
        Selection::One { initial: 1 },
        "LMS stays selected"
    );
    assert_eq!(
        ui.screens[1].choices.len(),
        4,
        "LMS offers only SQLx databases"
    );
    assert_eq!(crumbs[7], "Step 4 of 5 · Features");
    assert_eq!(
        ui.screens[7].selection,
        Selection::Many {
            checked: vec![false, true, false, false, false, false, false, false, false]
        }
    );
    assert_eq!(plan.blueprint, BLANK_BLUEPRINT_ID);
    assert_eq!(plan.database, Database::None);
    assert!(!plan.redis && !plan.db_needed());
}

#[test]
fn switching_to_a_product_blueprint_restores_a_supported_database() {
    let mut ui = FakeUi::scripted(
        &[],
        vec![
            Answer::One(0),       // Blank
            Answer::One(1),       // JSON API
            Answer::One(5),       // No database
            Answer::Many(vec![]), // no features
            Answer::Back,         // review → Features
            Answer::Back,         // → Database
            Answer::Back,         // → Application
            Answer::Back,         // → Blueprint
            Answer::One(1),       // LMS
            Answer::One(3),       // MariaDB
            Answer::Many(vec![]),
            Answer::One(0),
        ],
    );
    let plan = created(run_with(
        NewProjectRequest {
            name: Some("academy"),
            ..NewProjectRequest::default()
        },
        &mut ui,
    ));
    assert_eq!(plan.blueprint, LMS_BLUEPRINT_ID);
    assert!(!plan.api, "the Blank-only API answer does not survive");
    assert_eq!(plan.database, Database::Provider("MariaDB"));
    let database = &ui.screens[9];
    assert_eq!(
        database.selection,
        Selection::One { initial: 0 },
        "SQLite replaced none"
    );
}

#[test]
fn cancel_and_invalid_answers_never_create_anything() {
    let mut ui = FakeUi::scripted(
        &[],
        vec![
            Answer::One(0),
            Answer::One(0),
            Answer::One(0),
            Answer::Many(vec![]),
            Answer::One(2),
        ],
    );
    let outcome = run_with(
        NewProjectRequest {
            name: Some("later"),
            ..NewProjectRequest::default()
        },
        &mut ui,
    )
    .expect("cancelled");
    assert_eq!(outcome, Outcome::Cancelled);

    for answer in [Answer::One(usize::MAX), Answer::Many(vec![0])] {
        let mut ui = FakeUi::scripted(&[], vec![answer]);
        assert!(
            run_with(
                NewProjectRequest {
                    name: Some("broken"),
                    ..NewProjectRequest::default()
                },
                &mut ui,
            )
            .is_err()
        );
    }

    let mut ui = FakeUi::scripted(&[], vec![Answer::One(0), Answer::One(0), Answer::One(0)]);
    let ignored = run_with(
        NewProjectRequest {
            name: Some("bounded"),
            ..NewProjectRequest::default()
        },
        &mut ui,
    );
    assert!(ignored.is_err(), "UI errors propagate instead of creating");

    let mut missing_name = FakeUi::default();
    assert!(run_with(NewProjectRequest::default(), &mut missing_name).is_err());
}

#[test]
fn out_of_range_feature_indices_are_ignored() {
    let mut ui = FakeUi::scripted(
        &[],
        vec![
            Answer::One(0),
            Answer::One(0),
            Answer::Many(vec![usize::MAX]),
            Answer::One(0),
        ],
    );
    let plan = created(run_with(
        NewProjectRequest {
            name: Some("bounded"),
            blueprint: Some(BLANK_BLUEPRINT_ID),
            ..NewProjectRequest::default()
        },
        &mut ui,
    ));
    assert!(plan.features().is_empty());
}

#[test]
fn a_dry_run_review_finishes_without_creating() {
    let mut ui = FakeUi::scripted(&[], vec![Answer::Many(vec![]), Answer::One(0)]);
    let plan = created(run_with(
        NewProjectRequest {
            name: Some("preview"),
            blueprint: Some(SAAS_BLUEPRINT_ID),
            skip_initial_migration: true,
            dry_run: true,
            options: ProjectScaffoldOptions {
                database: Some("Sqlite"),
                ..ProjectScaffoldOptions::default()
            },
        },
        &mut ui,
    ));
    assert!(plan.skip_initial_migration);
    let review = ui.screens.last().expect("review");
    assert_eq!(review.choices[0].label, "Finish the dry run");
    assert!(
        texts(&review.body)
            .iter()
            .any(|line| line.contains("no commands; files only"))
    );
}

#[test]
fn the_library_wizard_does_not_offer_packaging_it_cannot_apply() {
    let request = NewProjectRequest {
        name: Some("library"),
        blueprint: Some(BLANK_BLUEPRINT_ID),
        options: ProjectScaffoldOptions {
            database: Some("Sqlite"),
            ..ProjectScaffoldOptions::default()
        },
        ..NewProjectRequest::default()
    };
    let requested = requested_integrations(&request.options);
    let plan = initial_plan(&request, &requested);
    let mut ui = FakeUi::scripted(
        &[],
        vec![Answer::One(0), Answer::Many(vec![]), Answer::One(0)],
    );
    created(run(
        interactive_setup(&request, plan, false, 3000),
        &mut ui,
        fake_preview,
    ));
    let offered: Vec<&str> = ui.screens[1]
        .choices
        .iter()
        .map(|choice| choice.label.as_str())
        .collect();
    assert!(!offered.contains(&"Dockerfile") && !offered.contains(&"Nix flake"));
}

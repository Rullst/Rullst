use super::*;
use std::{collections::VecDeque, io};

#[derive(Default)]
struct FakeWizardUi {
    inputs: VecDeque<String>,
    selections: VecDeque<usize>,
    confirmations: VecDeque<bool>,
    multiple: VecDeque<Vec<usize>>,
    prompts: Vec<String>,
}

impl ProjectWizardUi for FakeWizardUi {
    fn input(&mut self, prompt: &str) -> WizardResult<String> {
        self.prompts.push(prompt.to_string());
        self.inputs.pop_front().ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "missing fake input").into()
        })
    }

    fn select(&mut self, prompt: &str, choices: &[String]) -> WizardResult<usize> {
        self.prompts.push(format!("{prompt}|{}", choices.join("|")));
        self.selections.pop_front().ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "missing fake selection").into()
        })
    }

    fn confirm(&mut self, prompt: &str, default: bool) -> WizardResult<bool> {
        self.prompts.push(format!("{prompt}|default={default}"));
        self.confirmations.pop_front().ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "missing fake confirmation").into()
        })
    }

    fn multi_select(&mut self, prompt: &str, choices: &[String]) -> WizardResult<Vec<usize>> {
        self.prompts.push(format!("{prompt}|{}", choices.join("|")));
        self.multiple.pop_front().ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "missing fake multi-selection").into()
        })
    }
}

#[test]
fn positional_name_does_not_skip_interactive_project_profile() {
    assert!(should_prompt_project_profile(true, false));
    assert!(should_prompt_project_profile(false, false));
    assert!(!should_prompt_project_profile(true, true));
}

#[test]
fn turso_primary_is_offered_only_for_its_supported_blank_profile() {
    assert!(
        primary_database_options(BLANK_BLUEPRINT_ID)
            .iter()
            .any(|(_, provider)| *provider == "Turso")
    );
    for blueprint in [
        LMS_BLUEPRINT_ID,
        SAAS_BLUEPRINT_ID,
        BLOG_BLUEPRINT_ID,
        PORTFOLIO_BLUEPRINT_ID,
        ERP_BLUEPRINT_ID,
    ] {
        assert!(
            primary_database_options(blueprint)
                .iter()
                .all(|(_, provider)| *provider != "Turso")
        );
    }
}

#[test]
fn deterministic_wizard_preserves_requested_persistence_features() {
    let selected = [
        PolyglotIntegration::Turso,
        PolyglotIntegration::MongoDb,
        PolyglotIntegration::DuckDb,
        PolyglotIntegration::SurrealDb,
        PolyglotIntegration::Qdrant,
    ];
    let options = run_project_wizard_with_blueprint(
        Some("polyglot-app"),
        ProjectScaffoldOptions {
            use_defaults: true,
            database: Some("MariaDB"),
            ..ProjectScaffoldOptions::default()
        },
        &selected,
        Some(BLANK_BLUEPRINT_ID),
    )
    .expect("deterministic wizard");

    assert_eq!(options.db_provider, "MariaDB");
    assert_eq!(options.polyglot_integrations, selected);
    assert!(options.turso);
}

#[test]
fn v12_keeps_all_optional_storage_add_ons_and_omits_selected_ones() {
    let all_options = available_optional_storage_options(&[]);
    assert_eq!(all_options.len(), OPTIONAL_STORAGE_OPTIONS.len());
    assert_eq!(
        all_options
            .iter()
            .map(|(_, integration)| *integration)
            .collect::<Vec<_>>(),
        OPTIONAL_STORAGE_OPTIONS
            .iter()
            .map(|(_, integration)| *integration)
            .collect::<Vec<_>>()
    );
    assert!(
        all_options[0]
            .0
            .contains("application integration remains explicit in v12")
    );

    let without_turso = available_optional_storage_options(&[PolyglotIntegration::Turso]);
    assert_eq!(without_turso.len(), OPTIONAL_STORAGE_OPTIONS.len() - 1);
    assert!(
        without_turso
            .iter()
            .all(|(_, integration)| *integration != PolyglotIntegration::Turso)
    );
}

#[test]
fn deterministic_wizard_locks_the_supported_v12_application_profile() {
    let options = run_project_wizard_with_blueprint(
        Some("profiled-app"),
        ProjectScaffoldOptions {
            use_defaults: true,
            database: Some("Postgres"),
            hot_reload: false,
            wants_ai: true,
            wants_redis: true,
            ..ProjectScaffoldOptions::default()
        },
        &[],
        Some(ERP_BLUEPRINT_ID),
    )
    .expect("deterministic build axes");

    assert_eq!(options.db_provider, "Postgres");
    assert_eq!(options.orm_pattern, V12_ORM_PATTERN);
    assert_eq!(options.frontend_engine, V12_FRONTEND_ENGINE);
    assert!(!options.hot_reload);
    assert!(options.wants_ai);
    assert!(options.wants_redis);
}

#[test]
fn impossible_deterministic_profiles_fail_instead_of_being_ignored() {
    let api_lms = run_project_wizard_with_blueprint(
        Some("invalid-api-lms"),
        ProjectScaffoldOptions {
            use_defaults: true,
            api: true,
            ..ProjectScaffoldOptions::default()
        },
        &[],
        Some(LMS_BLUEPRINT_ID),
    );
    assert!(api_lms.is_err());

    let no_database_lms = run_project_wizard_with_blueprint(
        Some("invalid-lms"),
        ProjectScaffoldOptions {
            use_defaults: true,
            no_database: true,
            ..ProjectScaffoldOptions::default()
        },
        &[],
        Some(LMS_BLUEPRINT_ID),
    );
    assert!(no_database_lms.is_err());

    let turso_hot_reload = run_project_wizard_with_blueprint(
        Some("invalid-edge"),
        ProjectScaffoldOptions {
            use_defaults: true,
            database: Some("Turso"),
            hot_reload: true,
            ..ProjectScaffoldOptions::default()
        },
        &[PolyglotIntegration::Turso],
        Some(BLANK_BLUEPRINT_ID),
    );
    assert!(turso_hot_reload.is_err());

    for database in ["MongoDB", "sqlite", "TURSO"] {
        assert!(
            run_project_wizard_with_blueprint(
                Some("invalid-provider"),
                ProjectScaffoldOptions {
                    use_defaults: true,
                    database: Some(database),
                    ..ProjectScaffoldOptions::default()
                },
                &[],
                Some(BLANK_BLUEPRINT_ID),
            )
            .is_err()
        );
    }
    assert!(
        run_project_wizard_with_blueprint(
            Some("invalid-blueprint"),
            ProjectScaffoldOptions {
                use_defaults: true,
                ..ProjectScaffoldOptions::default()
            },
            &[],
            Some(usize::MAX),
        )
        .is_err()
    );
}

#[test]
fn public_wizard_wrapper_preserves_default_and_turso_requests() {
    let default =
        run_project_wizard(Some("default-app"), false, true, false).expect("default public wizard");
    assert_eq!(default.name, "default-app");
    assert!(!default.turso);
    assert!(default.polyglot_integrations.is_empty());

    let turso =
        run_project_wizard(Some("edge-app"), false, true, true).expect("Turso public wizard");
    assert!(turso.turso);
    assert_eq!(turso.polyglot_integrations, [PolyglotIntegration::Turso]);
}

#[test]
fn interactive_blank_wizard_validates_names_and_composes_the_full_profile() {
    let mut ui = FakeWizardUi {
        inputs: ["", "space name", "1number", "bad!", "learning_hub"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        selections: [BLANK_BLUEPRINT_ID, 1, 4].into(),
        confirmations: [true, true, false].into(),
        multiple: [vec![0, 1, 2, 3]].into(),
        ..FakeWizardUi::default()
    };

    let result =
        run_project_wizard_with_ui(None, ProjectScaffoldOptions::default(), &[], None, &mut ui)
            .expect("interactive blank profile");

    assert_eq!(result.name, "learning_hub");
    assert!(result.api);
    assert!(result.db_needed);
    assert_eq!(result.db_provider, "Turso");
    assert_eq!(result.orm_pattern, "Turso Active Record");
    assert!(result.wants_ai);
    assert!(!result.wants_redis);
    assert_eq!(
        result.polyglot_integrations,
        [
            PolyglotIntegration::Turso,
            PolyglotIntegration::MongoDb,
            PolyglotIntegration::DuckDb,
            PolyglotIntegration::SurrealDb,
            PolyglotIntegration::Qdrant,
        ]
    );
    assert!(
        ui.prompts
            .iter()
            .any(|prompt| prompt.contains("zero or more"))
    );
}

#[test]
fn interactive_nonblank_and_database_free_profiles_keep_explicit_boundaries() {
    let mut lms_ui = FakeWizardUi {
        selections: [LMS_BLUEPRINT_ID, 3].into(),
        confirmations: [false, true].into(),
        multiple: [vec![]].into(),
        ..FakeWizardUi::default()
    };
    let lms = run_project_wizard_with_ui(
        Some("academy"),
        ProjectScaffoldOptions::default(),
        &[],
        None,
        &mut lms_ui,
    )
    .expect("interactive LMS profile");
    assert_eq!(lms.blueprint_selection, LMS_BLUEPRINT_ID);
    assert_eq!(lms.db_provider, "MariaDB");
    assert!(lms.db_needed);
    assert!(!lms.api);
    assert!(!lms.wants_ai);
    assert!(lms.wants_redis);

    let mut no_database_ui = FakeWizardUi {
        selections: [BLANK_BLUEPRINT_ID, 0].into(),
        confirmations: [false, true, false].into(),
        multiple: [vec![]].into(),
        ..FakeWizardUi::default()
    };
    let no_database = run_project_wizard_with_ui(
        Some("static-app"),
        ProjectScaffoldOptions::default(),
        &[],
        None,
        &mut no_database_ui,
    )
    .expect("database-free blank profile");
    assert!(!no_database.db_needed);
    assert_eq!(no_database.db_provider, "Sqlite");
    assert!(no_database.wants_ai);
}

#[test]
fn interactive_wizard_rejects_invalid_choices_and_propagates_input_errors() {
    let mut invalid_database = FakeWizardUi {
        selections: [0, usize::MAX].into(),
        confirmations: [true].into(),
        ..FakeWizardUi::default()
    };
    assert!(
        run_project_wizard_with_ui(
            Some("invalid-db"),
            ProjectScaffoldOptions::default(),
            &[],
            Some(BLANK_BLUEPRINT_ID),
            &mut invalid_database,
        )
        .is_err()
    );

    let mut ignored_multi_index = FakeWizardUi {
        selections: [0, 0].into(),
        confirmations: [true, false, false].into(),
        multiple: [vec![usize::MAX]].into(),
        ..FakeWizardUi::default()
    };
    let result = run_project_wizard_with_ui(
        Some("bounded-multi"),
        ProjectScaffoldOptions::default(),
        &[],
        Some(BLANK_BLUEPRINT_ID),
        &mut ignored_multi_index,
    )
    .expect("out-of-range optional add-on is ignored");
    assert!(result.polyglot_integrations.is_empty());

    let mut missing_name = FakeWizardUi::default();
    assert!(
        run_project_wizard_with_ui(
            None,
            ProjectScaffoldOptions::default(),
            &[],
            None,
            &mut missing_name,
        )
        .is_err()
    );

    let mut missing_blueprint = FakeWizardUi::default();
    assert!(
        run_project_wizard_with_ui(
            Some("missing-choice"),
            ProjectScaffoldOptions::default(),
            &[],
            None,
            &mut missing_blueprint,
        )
        .is_err()
    );

    let mut api_nonblank = FakeWizardUi::default();
    assert!(
        run_project_wizard_with_ui(
            Some("api-lms"),
            ProjectScaffoldOptions {
                api: true,
                ..ProjectScaffoldOptions::default()
            },
            &[],
            Some(LMS_BLUEPRINT_ID),
            &mut api_nonblank,
        )
        .is_err()
    );
}

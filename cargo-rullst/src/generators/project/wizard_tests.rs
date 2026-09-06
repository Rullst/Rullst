use super::*;

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
            hot_reload: true,
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
    assert!(options.hot_reload);
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
}

use super::*;

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
fn deterministic_wizard_preserves_every_build_axis() {
    let options = run_project_wizard_with_blueprint(
        Some("profiled-app"),
        ProjectScaffoldOptions {
            use_defaults: true,
            database: Some("Postgres"),
            orm_pattern: Some("Hybrid"),
            frontend_engine: Some("Tera Templates"),
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
    assert_eq!(options.orm_pattern, "Hybrid");
    assert_eq!(options.frontend_engine, "Tera Templates");
    assert!(options.hot_reload);
    assert!(options.wants_ai);
    assert!(options.wants_redis);
}

#[test]
fn impossible_deterministic_profiles_fail_instead_of_being_ignored() {
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

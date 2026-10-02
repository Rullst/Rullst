use super::catalog::database_options;
use super::*;

fn defaults() -> ProjectScaffoldOptions {
    ProjectScaffoldOptions {
        use_defaults: true,
        ..ProjectScaffoldOptions::default()
    }
}

#[test]
fn turso_and_database_free_profiles_are_offered_only_for_the_blank_starter() {
    let blank = database_options(BLANK_BLUEPRINT_ID);
    assert!(
        blank
            .iter()
            .any(|option| option.database == Database::Provider("Turso"))
    );
    assert!(blank.iter().any(|option| option.database == Database::None));
    assert_eq!(blank[0].database, Database::Provider("Sqlite"));
    for blueprint in [
        LMS_BLUEPRINT_ID,
        SAAS_BLUEPRINT_ID,
        BLOG_BLUEPRINT_ID,
        PORTFOLIO_BLUEPRINT_ID,
        ERP_BLUEPRINT_ID,
    ] {
        let options = database_options(blueprint);
        assert_eq!(options.len(), 4);
        assert!(options.iter().all(|option| {
            !matches!(
                option.database,
                Database::None | Database::Provider("Turso")
            )
        }));
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
            database: Some("MariaDB"),
            ..defaults()
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
fn deterministic_wizard_locks_the_supported_v12_application_profile() {
    let options = run_project_wizard_with_blueprint(
        Some("profiled-app"),
        ProjectScaffoldOptions {
            database: Some("Postgres"),
            hot_reload: false,
            wants_ai: true,
            wants_redis: true,
            ..defaults()
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
fn deterministic_defaults_match_the_previous_blank_profile() {
    let options =
        run_project_wizard_with_blueprint(None, defaults(), &[], None).expect("bare --default");
    assert_eq!(options.name, "app");
    assert!(!options.api);
    assert_eq!(options.db_provider, "Sqlite");
    assert!(options.db_needed);
    assert_eq!(options.blueprint_selection, BLANK_BLUEPRINT_ID);
    assert!(!options.wants_ai && !options.wants_redis && !options.turso);
    assert!(options.polyglot_integrations.is_empty());

    let lean = run_project_wizard_with_blueprint(
        Some("lean"),
        ProjectScaffoldOptions {
            api: true,
            no_database: true,
            ..defaults()
        },
        &[],
        None,
    )
    .expect("database-free API");
    assert!(lean.api && !lean.db_needed);
    assert_eq!(lean.db_provider, "Sqlite");
    assert_eq!(lean.orm_pattern, V12_ORM_PATTERN);

    let edge = run_project_wizard_with_blueprint(
        Some("edge"),
        ProjectScaffoldOptions {
            database: Some("Turso"),
            ..defaults()
        },
        &[PolyglotIntegration::Turso],
        None,
    )
    .expect("Turso primary");
    assert_eq!(edge.orm_pattern, "Turso Active Record");
    assert_eq!(edge.polyglot_integrations, [PolyglotIntegration::Turso]);
}

#[test]
fn impossible_deterministic_profiles_fail_instead_of_being_ignored() {
    let api_lms = run_project_wizard_with_blueprint(
        Some("invalid-api-lms"),
        ProjectScaffoldOptions {
            api: true,
            ..defaults()
        },
        &[],
        Some(LMS_BLUEPRINT_ID),
    );
    assert!(api_lms.is_err());

    let no_database_lms = run_project_wizard_with_blueprint(
        Some("invalid-lms"),
        ProjectScaffoldOptions {
            no_database: true,
            ..defaults()
        },
        &[],
        Some(LMS_BLUEPRINT_ID),
    );
    assert!(no_database_lms.is_err());

    let turso_hot_reload = run_project_wizard_with_blueprint(
        Some("invalid-edge"),
        ProjectScaffoldOptions {
            database: Some("Turso"),
            hot_reload: true,
            ..defaults()
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
                    database: Some(database),
                    ..defaults()
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
            defaults(),
            &[],
            Some(usize::MAX)
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
fn without_a_terminal_the_wizard_never_prompts() {
    for request in [
        NewProjectRequest::default(),
        NewProjectRequest {
            name: Some("scripted"),
            blueprint: Some(SAAS_BLUEPRINT_ID),
            dry_run: true,
            ..NewProjectRequest::default()
        },
    ] {
        let error = plan_project(&request, &[], &Terminal::non_interactive(), true)
            .expect_err("no prompt without a terminal");
        let message = error.to_string();
        assert!(message.contains("interactive terminal"), "{message}");
        assert!(message.contains("--default"), "{message}");
    }

    let deterministic = NewProjectRequest {
        name: Some("scripted"),
        options: defaults(),
        dry_run: true,
        ..NewProjectRequest::default()
    };
    assert!(matches!(
        plan_project(&deterministic, &[], &Terminal::non_interactive(), true),
        Ok(Planned::Ready {
            reviewed: false,
            ..
        })
    ));
}

#[test]
fn requested_integrations_keep_the_manifest_order() {
    let options = ProjectScaffoldOptions {
        qdrant: true,
        mongodb: true,
        database: Some("Turso"),
        ..ProjectScaffoldOptions::default()
    };
    assert_eq!(
        requested_integrations(&options),
        [
            PolyglotIntegration::Turso,
            PolyglotIntegration::MongoDb,
            PolyglotIntegration::Qdrant
        ]
    );
    assert!(requested_integrations(&ProjectScaffoldOptions::default()).is_empty());
}

#[test]
fn profile_flags_lock_their_questions() {
    let locked = locked_questions(&NewProjectRequest {
        name: Some("x"),
        options: ProjectScaffoldOptions {
            api: true,
            wants_ai: true,
            buildah: true,
            ..ProjectScaffoldOptions::default()
        },
        ..NewProjectRequest::default()
    });
    assert!(locked.name && locked.blueprint && locked.application && locked.ai);
    assert!(locked.docker && !locked.database && !locked.redis && !locked.nix);

    for options in [
        ProjectScaffoldOptions {
            no_database: true,
            ..ProjectScaffoldOptions::default()
        },
        ProjectScaffoldOptions {
            database: Some("Turso"),
            ..ProjectScaffoldOptions::default()
        },
    ] {
        let locked = locked_questions(&NewProjectRequest {
            options,
            ..NewProjectRequest::default()
        });
        assert!(locked.blueprint && locked.database && !locked.application);
    }
    assert_eq!(
        locked_questions(&NewProjectRequest::default()),
        flow::Locked::default()
    );
}

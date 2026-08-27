//! Cheap contract coverage for every CLI command and blueprint variant.
//!
//! This test intentionally does not invoke nested Cargo builds. The focused
//! `generated_saas_check` suite runs the slower quality gate for representative
//! generated projects; compiling every variant remains separate roadmap work.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::path::{Component, Path};

use cargo_rullst::blueprints::{
    self, BLANK_BLUEPRINT_ID, BLOG_BLUEPRINT_ID, ERP_BLUEPRINT_ID, LMS_BLUEPRINT_ID,
    PORTFOLIO_BLUEPRINT_ID, SAAS_BLUEPRINT_ID,
};
use cargo_rullst::cli::Cli;
use cargo_rullst::generators::audit::scan_idor_vulnerabilities;
use cargo_rullst::generators::project::cargo_toml::build_cargo_toml;
use clap::CommandFactory;

const ORM_PATTERNS: [&str; 3] = ["Active Record", "Repository", "Hybrid"];
const FRONTEND_ENGINES: [&str; 5] = [
    "Zero-Bundle HTMX",
    "Wasm Island",
    "LiveView",
    "Pico CSS",
    "Tera Template",
];

#[derive(Clone, Copy)]
struct BlueprintSpec {
    id: usize,
    key: &'static str,
}

const BLUEPRINTS: [BlueprintSpec; 6] = [
    BlueprintSpec {
        id: BLANK_BLUEPRINT_ID,
        key: "blank",
    },
    BlueprintSpec {
        id: LMS_BLUEPRINT_ID,
        key: "lms",
    },
    BlueprintSpec {
        id: SAAS_BLUEPRINT_ID,
        key: "saas",
    },
    BlueprintSpec {
        id: BLOG_BLUEPRINT_ID,
        key: "blog",
    },
    BlueprintSpec {
        id: PORTFOLIO_BLUEPRINT_ID,
        key: "portfolio",
    },
    BlueprintSpec {
        id: ERP_BLUEPRINT_ID,
        key: "erp",
    },
];

#[allow(clippy::too_many_arguments)]
fn manifest_for(
    spec: BlueprintSpec,
    api: bool,
    hot_reload: bool,
    db_needed: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    match spec.id {
        BLANK_BLUEPRINT_ID => blueprints::blank::file_manifest(
            "matrix-app",
            "matrix_app",
            api,
            hot_reload,
            db_needed,
            orm_pattern,
            frontend_engine,
        ),
        LMS_BLUEPRINT_ID => {
            blueprints::lms::file_manifest("matrix_app", hot_reload, orm_pattern, frontend_engine)
        }
        SAAS_BLUEPRINT_ID => {
            blueprints::saas::file_manifest("matrix_app", hot_reload, orm_pattern, frontend_engine)
        }
        BLOG_BLUEPRINT_ID => {
            blueprints::blog::file_manifest("matrix_app", hot_reload, orm_pattern, frontend_engine)
        }
        PORTFOLIO_BLUEPRINT_ID => blueprints::portfolio::file_manifest(
            "matrix_app",
            hot_reload,
            orm_pattern,
            frontend_engine,
        ),
        ERP_BLUEPRINT_ID => {
            blueprints::erp::file_manifest("matrix_app", hot_reload, orm_pattern, frontend_engine)
        }
        _ => panic!("unregistered blueprint ID {}", spec.id),
    }
}

fn assert_safe_parseable_manifest(
    case: &str,
    manifest: &[(&'static str, String)],
) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for (relative, contents) in manifest {
        let path = Path::new(relative);
        assert!(
            !path.is_absolute(),
            "{case}: absolute output path {relative}"
        );
        assert!(
            path.components()
                .all(|part| matches!(part, Component::Normal(_))),
            "{case}: non-normal output path {relative}"
        );
        assert!(
            paths.insert((*relative).to_string()),
            "{case}: duplicate output path {relative}"
        );

        if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            syn::parse_file(contents).unwrap_or_else(|error| {
                panic!("{case}: generated Rust at {relative} does not parse: {error}")
            });
        }
    }
    assert!(
        paths.contains("src/main.rs") || paths.contains("src/lib.rs"),
        "{case}: blueprint has no Rust entry point"
    );
    paths
}

#[test]
fn every_blueprint_variant_has_safe_paths_valid_rust_and_valid_manifest() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let mut cases = 0usize;

    for spec in BLUEPRINTS {
        let api_modes: &[bool] = if spec.id == BLANK_BLUEPRINT_ID {
            &[false, true]
        } else {
            &[false]
        };
        let db_modes: &[bool] = if spec.id == BLANK_BLUEPRINT_ID {
            &[false, true]
        } else {
            &[true]
        };

        for api in api_modes {
            for db_needed in db_modes {
                for hot_reload in [false, true] {
                    for orm_pattern in ORM_PATTERNS {
                        for frontend_engine in FRONTEND_ENGINES {
                            let case = format!(
                                "{}:api={api}:db={db_needed}:hot={hot_reload}:orm={orm_pattern}:frontend={frontend_engine}",
                                spec.key
                            );
                            let files = manifest_for(
                                spec,
                                *api,
                                hot_reload,
                                *db_needed,
                                orm_pattern,
                                frontend_engine,
                            );
                            assert_safe_parseable_manifest(&case, &files);
                            if spec.id != BLANK_BLUEPRINT_ID {
                                let rust_sources = files
                                    .iter()
                                    .filter(|(path, _)| path.ends_with(".rs"))
                                    .map(|(_, contents)| contents.as_str())
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                assert!(
                                    rust_sources.contains(
                                        "NexusAuthPolicy::local_development_or_basic_from_env()"
                                    ),
                                    "{case}: Nexus local/release access policy drifted"
                                );
                                assert!(
                                    rust_sources.contains(
                                        "#[cfg(debug_assertions)]\n    {\n        rullst::runtime::spawn"
                                    ) && rust_sources.contains("run_studio(5555)"),
                                    "{case}: Studio must be a debug-build-only local service"
                                );
                            }

                            let cargo_toml = build_cargo_toml(
                                "matrix-app",
                                hot_reload,
                                *db_needed,
                                "Sqlite",
                                false,
                                false,
                                spec.id,
                                frontend_engine,
                                workspace,
                            )
                            .unwrap_or_else(|error| {
                                panic!("{case}: could not generate Cargo.toml: {error}")
                            });
                            let document: toml::Value =
                                toml::from_str(&cargo_toml).unwrap_or_else(|error| {
                                    panic!("{case}: invalid Cargo.toml: {error}")
                                });
                            assert_eq!(
                                document["package"]["name"].as_str(),
                                Some("matrix-app"),
                                "{case}: package identity drifted"
                            );
                            assert!(
                                document.get("workspace").is_some(),
                                "{case}: generated project must be isolated from the parent workspace"
                            );
                            cases += 1;
                        }
                    }
                }
            }
        }
    }

    assert_eq!(cases, 270, "the documented structural matrix changed");
}

#[test]
fn apply_materializes_each_public_blueprint_without_path_drift() {
    for spec in BLUEPRINTS {
        let root = std::env::temp_dir().join(format!(
            "rullst-blueprint-contract-{}-{}",
            spec.key,
            rand::random::<u64>()
        ));
        let expected = manifest_for(
            spec,
            false,
            false,
            spec.id != BLANK_BLUEPRINT_ID,
            "Active Record",
            "Zero-Bundle HTMX",
        );

        blueprints::apply(
            spec.id,
            &root,
            "matrix-app",
            "matrix_app",
            false,
            false,
            spec.id != BLANK_BLUEPRINT_ID,
            "Active Record",
            "Zero-Bundle HTMX",
        )
        .unwrap_or_else(|error| panic!("{}: apply failed: {error}", spec.key));

        for (relative, contents) in expected {
            let actual = std::fs::read_to_string(root.join(relative))
                .unwrap_or_else(|error| panic!("{}:{relative}: {error}", spec.key));
            assert_eq!(actual, contents, "{}:{relative}: write drift", spec.key);
        }
        let (idor_count, idor_findings) = scan_idor_vulnerabilities(&root.join("src"));
        assert_eq!(
            idor_count, 0,
            "{}: generated parameterized routes lack an auditable access boundary: {idor_findings:#?}",
            spec.key
        );
        std::fs::remove_dir_all(&root)
            .unwrap_or_else(|error| panic!("{}: temp cleanup failed: {error}", spec.key));
    }
}

#[test]
fn workspace_parameterized_routes_have_explicit_access_boundaries() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let (idor_count, idor_findings) = scan_idor_vulnerabilities(workspace);
    assert_eq!(
        idor_count, 0,
        "workspace parameterized-route audit failed: {idor_findings:#?}"
    );
}

#[test]
fn extracted_rust_templates_parse_after_substitution() {
    let templates = [
        (
            "auth controller",
            include_str!("../src/generators/auth/auth_controller.rs.template").to_string(),
        ),
        (
            "billing controller",
            include_str!("../src/generators/billing_controller.rs.template")
                .replace("__FOREIGN_KEY__", "user_id"),
        ),
        (
            "billing page",
            include_str!("../src/generators/billing_page.rs.template").to_string(),
        ),
        (
            "CORS middleware",
            include_str!("../src/generators/cors_middleware.rs.template").to_string(),
        ),
        (
            "JWT middleware",
            include_str!("../src/generators/jwt_middleware.rs.template").to_string(),
        ),
        (
            "Wasm island",
            include_str!("../src/generators/island.rs.template")
                .replace("__MODULE_NAME__", "contract_island")
                .replace("__TYPE_NAME__", "ContractIsland"),
        ),
        (
            "worker",
            cargo_rullst::generators::worker::render_worker_source("contract"),
        ),
    ];

    for (name, source) in templates {
        syn::parse_file(&source)
            .unwrap_or_else(|error| panic!("{name} template does not parse: {error}"));
    }
}

#[test]
fn public_cli_command_inventory_is_explicit_and_complete() {
    let actual = Cli::command()
        .get_subcommands()
        .map(|command| command.get_name().to_string())
        .filter(|name| name != "help")
        .collect::<BTreeSet<_>>();
    let expected = [
        "audit",
        "auth",
        "build",
        "build:client",
        "dash",
        "db:migrate",
        "db:rollback",
        "db:seed",
        "db:status",
        "deploy",
        "dev",
        "doctor",
        "dockerize",
        "eject",
        "foundry:deploy",
        "foundry:init",
        "generate:ai-context",
        "generate:buildah",
        "generate:diagram",
        "generate:models",
        "generate:openapi",
        "generate:ts",
        "hook:install",
        "inspect",
        "make:billing",
        "make:chat-session",
        "make:controller",
        "make:cors",
        "make:grpc",
        "make:iot",
        "make:island",
        "make:jwt",
        "make:k8s",
        "make:live",
        "make:mail",
        "make:mfa",
        "make:middleware",
        "make:migration",
        "make:migration:auto",
        "make:model",
        "make:omni",
        "make:resource",
        "make:scalar",
        "make:worker",
        "new",
        "nixify",
        "omni",
        "pkg",
        "studio",
        "upgrade",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<BTreeSet<_>>();

    assert_eq!(actual, expected, "update the scaffold validation inventory");
}

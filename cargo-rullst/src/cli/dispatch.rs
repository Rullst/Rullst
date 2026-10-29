// src/cli/dispatch.rs — Routes each parsed command to its generator function.
#![cfg_attr(mutants, mutants::skip)]

use std::path::{Path, PathBuf};

use super::{BlueprintChoice, Commands, DatabaseChoice};
use crate::generators::{
    auth::scaffold_auth,
    billing::scaffold_billing_system,
    build::{UpgradeOptions, run_build_client, run_production_build, run_upgrade},
    controller::create_new_controller,
    cors_jwt::{create_cors_middleware, create_jwt_middleware},
    db::run_project_db_command,
    inspect::inspect_project,
    introspect::generate_models_from_db,
    mail::{MailableKind, create_new_mailable},
    middleware::create_new_middleware,
    migration::create_new_migration,
    model::create_new_model,
    openapi::generate_openapi_spec,
    output_guard::{reject_existing, reject_symlink},
    project::{ProjectScaffoldOptions, create_new_project_with_cli_options},
    resource::create_new_resource,
    worker::create_new_worker,
};

/// Packaging files are application-owned once generated; regeneration is explicit.
const REGENERATE_HINT: &str = "; move them aside to regenerate the template";

/// Central command dispatcher. Routes each CLI command to its generator function.
pub fn run_cli_command(command: &Commands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::New {
            name,
            api,
            docker,
            default,
            blueprint,
            database,
            no_database,
            hot_reload,
            ai,
            redis,
            skip_initial_migration,
            turso,
            mongodb,
            duckdb,
            surrealdb,
            qdrant,
        } => {
            let starter = blueprint.map(BlueprintChoice::starter).transpose()?;
            create_new_project_with_cli_options(
                name.as_deref(),
                ProjectScaffoldOptions {
                    api: *api || blueprint.is_some_and(BlueprintChoice::is_api),
                    docker: *docker,
                    use_defaults: *default,
                    turso: *turso,
                    mongodb: *mongodb,
                    duckdb: *duckdb,
                    surrealdb: *surrealdb,
                    qdrant: *qdrant,
                    database: database.map(DatabaseChoice::provider),
                    no_database: *no_database,
                    hot_reload: *hot_reload,
                    wants_ai: *ai,
                    wants_redis: *redis,
                },
                starter,
                *skip_initial_migration,
            )?;
        }
        Commands::MakeController { name, api } => {
            create_new_controller(name, *api)?;
        }
        Commands::MakeModel { name, migration } => {
            create_new_model(name, *migration)?;
        }
        Commands::MakeResource { name, api } => {
            create_new_resource(name, *api)?;
        }
        Commands::MakeMiddleware { name } => {
            create_new_middleware(name)?;
        }
        Commands::DbMigrate => {
            run_project_db_command("db:migrate")?;
        }
        Commands::DbRollback => {
            run_project_db_command("db:rollback")?;
        }
        Commands::DbStatus => {
            run_project_db_command("db:status")?;
        }
        Commands::DbSeed => {
            run_project_db_command("db:seed")?;
        }
        Commands::MakeMigration { name } => {
            create_new_migration(name)?;
        }
        Commands::MakeMigrationAuto => {
            tokio::runtime::Runtime::new()?
                .block_on(crate::generators::migration::create_auto_migration())?;
        }
        Commands::Auth => {
            // Nothing written (the scaffold already exists) refreshes nothing.
            if scaffold_auth()? {
                crate::generators::ai_context::refresh_after_scaffold();
                crate::generators::diagram::refresh_after_scaffold();
            }
        }
        Commands::MakeBilling { model } => {
            scaffold_billing_system(model)?;
        }
        Commands::MakeChatSession => {
            crate::generators::chat::scaffold_chat_session()?;
        }
        Commands::MakeIot { name } => {
            crate::generators::iot::run_make_iot(name)?;
        }
        Commands::MakeMail {
            name,
            welcome,
            reset,
            otp,
            invoice,
        } => {
            let kind = match (*welcome, *reset, *otp, *invoice) {
                (false, false, false, false) => MailableKind::Custom,
                (true, false, false, false) => MailableKind::Welcome,
                (false, true, false, false) => MailableKind::Reset,
                (false, false, true, false) => MailableKind::Otp,
                (false, false, false, true) => MailableKind::Invoice,
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "make:mail accepts at most one template flag",
                    )
                    .into());
                }
            };
            create_new_mailable(name, kind)?;
        }
        Commands::MakeMailInvoice { name } => {
            create_new_mailable(name, MailableKind::PaymentReceipt)?;
        }
        Commands::MakeMailDunning { name } => {
            create_new_mailable(name, MailableKind::Dunning)?;
        }
        Commands::Dockerize => {
            // `.dockerignore` is only created when absent; never write through a link.
            reject_symlink(Path::new(".dockerignore"))?;
            reject_existing(
                "Dockerfile",
                &[PathBuf::from("Dockerfile")],
                REGENERATE_HINT,
            )?;
            // The binary and image are named after `[package].name`, as in `deploy`.
            let proj_name = crate::generators::platform_name::package_name(Path::new("Cargo.toml"))
                .unwrap_or_else(|| "app".to_string());
            crate::generators::project::generate_docker_files(
                std::path::Path::new("."),
                &proj_name,
                None,
                None,
            )?;
        }
        Commands::MakeCors => {
            create_cors_middleware()?;
        }
        Commands::MakeJwt => {
            create_jwt_middleware()?;
        }
        Commands::GenerateOpenapi => {
            generate_openapi_spec()?;
        }
        Commands::GenerateTs => {
            crate::generators::ts::generate_ts_sdk()?;
        }
        Commands::GenerateDiagram => {
            println!("Generating Schema Visualizer...");
            crate::generators::diagram::generate_mermaid_diagram(None)?;
            println!("Diagram generated successfully at diagram.md");
        }
        Commands::GenerateModels {
            driver,
            url,
            output,
        } => {
            generate_models_from_db(driver, url, output)?;
        }
        Commands::GenerateAiContext => {
            crate::generators::ai_context::generate_ai_context(None)?;
        }
        Commands::MakeWorker { name } => {
            create_new_worker(name)?;
        }
        Commands::MakeIsland { name } => {
            crate::generators::island::create_new_island(name)?;
        }
        Commands::Upgrade {
            to,
            dry_run,
            json,
            keep_on_failure,
            restore,
            all_rules,
        } => {
            run_upgrade(UpgradeOptions {
                target: to.clone(),
                dry_run: *dry_run,
                json: *json,
                keep_on_failure: *keep_on_failure,
                restore: restore.clone(),
                all_rules: *all_rules,
            })?;
        }
        Commands::Dev { ts_sync } => {
            crate::generators::dev::run_dev(false, *ts_sync)?;
        }
        Commands::Pkg { action, name } => match (action.as_str(), name) {
            ("add", Some(pkg_name)) => {
                crate::pkg::pkg_add(pkg_name)?;
            }
            ("add", None) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "please specify a package name (e.g. 'cargo rullst pkg add rullst-auth')",
                )
                .into());
            }
            ("list", _) => {
                crate::pkg::pkg_list()?;
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown pkg action '{action}'; use 'add' or 'list'"),
                )
                .into());
            }
        },
        Commands::Dash => {
            crate::generators::dev::run_dev_server(true)?;
        }
        Commands::Studio { remote: true, port } => {
            super::studio::run_remote(*port)?;
        }
        Commands::Studio { remote: false, .. } => {
            run_project_db_command("studio")?;
        }
        Commands::BuildClient { debug } => {
            run_build_client(*debug)?;
        }
        Commands::Build { debug } => {
            run_production_build(!*debug)?;
        }
        Commands::Inspect { target } => {
            inspect_project(target.as_deref())?;
        }
        Commands::Audit {
            ai,
            compliance,
            idor,
            geiger,
            sbom,
            audit_ignore,
            network,
        } => {
            if audit_ignore.is_empty() {
                crate::generators::audit::run_security_audit(
                    *ai,
                    *compliance,
                    *idor,
                    *geiger,
                    *sbom,
                    *network,
                )?;
            } else {
                crate::generators::audit::run_security_audit_with_exceptions(
                    *ai,
                    *compliance,
                    *idor,
                    *geiger,
                    *sbom,
                    audit_ignore,
                    *network,
                )?;
            }
        }
        Commands::HookInstall => {
            crate::generators::hook::install_git_pre_commit_hook()?;
        }
        Commands::Doctor { fix } => {
            crate::generators::doctor::run_doctor(*fix)?;
        }
        Commands::AcademyDoctor { evidence, json } => {
            crate::generators::academy_doctor::run_academy_doctor(evidence.as_deref(), *json)?;
        }
        Commands::Eject { force, output } => {
            crate::generators::eject::run_eject_project(*force, output.as_deref())?;
        }
        Commands::MakeK8s => {
            crate::generators::k8s::generate_k8s_manifests()?;
        }
        Commands::MakeMfa => {
            crate::generators::auth::mfa::scaffold_mfa_system()?;
        }
        Commands::MakeScalar => {
            crate::generators::scalar::generate_scalar_docs()?;
        }
        Commands::MakeLive { name } => {
            crate::generators::live::create_new_live_component(name)?;
        }
        Commands::Deploy { platform } => {
            crate::generators::deploy::run_deploy(platform.as_deref())?;
        }
    }

    // Keep the generated project context and ER diagram current after scaffolds;
    // both refreshes are best effort and never replace hand-written files.
    match command {
        Commands::MakeController { .. }
        | Commands::MakeModel { .. }
        | Commands::MakeMiddleware { .. }
        | Commands::MakeWorker { .. }
        | Commands::MakeIsland { .. }
        | Commands::MakeBilling { .. }
        | Commands::MakeCors
        | Commands::MakeJwt => {
            crate::generators::ai_context::refresh_after_scaffold();
            crate::generators::diagram::refresh_after_scaffold();
        }
        _ => {}
    }

    Ok(())
}

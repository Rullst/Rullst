//! `cargo rullst make:oauth <provider>`: sign-in with Google, GitHub,
//! Microsoft or Apple for the `make:auth`, SaaS and LMS account model (v13
//! preview).
//!
//! The generated code keeps login challenges in the application database
//! through `rullst::connect::challenge`, links provider identities in one
//! `oauth_identities` table keyed by provider and subject, and signs
//! accounts in with the same session-epoch cookie as the password login. The
//! first provider creates the shared flow, the table and its migration; each
//! further provider adds only its module, routes, login link and settings.
//! Each provider is one [`providers::ProviderSpec`].
use super::consumer_files::{self as writes, Edit};
use clap::{Arg, ArgMatches, Command};
use providers::ProviderSpec;
use std::io;
use std::path::Path;

mod edits;
mod project;
mod providers;

const CONTROLLER_TEMPLATE: &str = include_str!("controller.rs.template");
const IDENTITY_TEMPLATE: &str = include_str!("identity.rs.template");
const MIGRATION_TEMPLATE: &str = include_str!("migration.rs.template");
/// File-name suffix of the identities migration.
const MIGRATION_SUFFIX: &str = "_create_oauth_identities_table.rs";
const UNKNOWN_PROVIDER: &str = "make:oauth supports only: google, github, microsoft, apple";

pub(crate) fn command() -> Command {
    Command::new("make:oauth")
        .about("Add sign-in with an OAuth/OpenID Connect provider to the make:auth, SaaS or LMS accounts (v13 preview)")
        .arg(
            Arg::new("provider")
                .required(true)
                .value_parser(providers::names())
                .help("The identity provider: google, github, microsoft or apple"),
        )
}

pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let name = matches
        .get_one::<String>("provider")
        .ok_or_else(|| invalid("the provider is required"))?;
    let root = std::env::current_dir()?;
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    let plan = plan(&root, name, &timestamp)?;
    writes::apply(&plan.edits)?;
    for note in plan.notes {
        println!("{note}");
    }
    Ok(())
}

/// The edits for one provider, and what to tell the developer afterwards.
struct Plan {
    edits: Vec<Edit>,
    notes: Vec<String>,
}

fn plan(root: &Path, name: &str, timestamp: &str) -> Result<Plan, Box<dyn std::error::Error>> {
    let provider = providers::find(name).ok_or_else(|| invalid(UNKNOWN_PROVIDER))?;
    if timestamp.len() != 14 || !timestamp.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid("the migration timestamp must be 14 digits").into());
    }
    let root = &writes::project_root(root)?;
    let database = project::check(root)?;
    let mut edits = Vec::new();
    let callback_method = if provider.form_post { "POST" } else { "GET" };
    let mut notes = vec![format!(
        "{} sign-in added: GET /auth/{name} starts it and {callback_method} /auth/{name}/callback finishes it.",
        provider.label
    )];

    let oauth_dir = root.join("src/controllers/oauth");
    let first_provider = !oauth_dir.join("mod.rs").try_exists()?;
    if first_provider {
        if project::has_migration(root, MIGRATION_SUFFIX)? {
            return Err(invalid(
                "an oauth_identities migration already exists; integrate the provider manually",
            )
            .into());
        }
        let controller = CONTROLLER_TEMPLATE
            .replace("__PROVIDER__", provider.name)
            .replace("__CHALLENGE_DATABASE__", database.sqlx_type());
        edits.push(Edit::create(oauth_dir.join("mod.rs"), rust(&controller)?)?);
        edits.push(Edit::create(
            root.join("src/models/oauth_identity.rs"),
            rust(IDENTITY_TEMPLATE)?,
        )?);
        let migration = format!("m{timestamp}{}", MIGRATION_SUFFIX.trim_end_matches(".rs"));
        let source = MIGRATION_TEMPLATE.replace("__MIGRATION_NAME__", &migration);
        let migrations = root.join("src/migrations");
        edits.push(Edit::create(
            migrations.join(format!("{migration}.rs")),
            rust(&source)?,
        )?);
        edits.push(edits::migrations_mod(&migrations, &migration)?);
        edits.push(edits::register_module(
            &root.join("src/controllers/mod.rs"),
            "oauth",
        )?);
        edits.push(edits::register_module(
            &root.join("src/models/mod.rs"),
            "oauth_identity",
        )?);
    } else {
        edits.push(edits::register_module(
            &oauth_dir.join("mod.rs"),
            provider.name,
        )?);
    }
    edits.push(Edit::create(
        oauth_dir.join(format!("{}.rs", provider.name)),
        rust(provider.template)?,
    )?);
    edits.push(edits::manifest(&root.join("Cargo.toml"), database)?);

    match edits::router(root, provider)? {
        Some(edit) => edits.push(edit),
        None if provider.form_post => notes.push(format!(
            "Mount the routes yourself, behind the CSRF layer: call controllers::oauth::{name}::initialize()? where the router is built, then route GET /auth/{name} to controllers::oauth::{name}::start, and POST /auth/{name}/callback to controllers::oauth::{name}::callback with GET to controllers::oauth::{name}::mock_callback, each with controllers::auth_controller::credential_rate_limit."
        )),
        None => notes.push(format!(
            "Mount the routes yourself, behind the CSRF layer: call controllers::oauth::{name}::initialize()? where the router is built, then route GET /auth/{name} to controllers::oauth::{name}::start and GET /auth/{name}/callback to controllers::oauth::{name}::callback, each with controllers::auth_controller::credential_rate_limit."
        )),
    }
    if provider.form_post
        && let Some(edit) = edits::csrf_exemption(root, &format!("/auth/{name}/callback"))?
    {
        edits.push(edit);
    }
    match edits::login_page(root, provider)? {
        Some(edit) => edits.push(edit),
        None => notes.push(format!(
            "Add {{crate::controllers::oauth::{name}::login_link()}} to your login page."
        )),
    }
    if let Some(edit) = edits::env_example(root, provider)? {
        edits.push(edit);
    }
    notes.push(format!(
        "Set {}, and register <RULLST_PUBLIC_ORIGIN>/auth/{name}/callback (or {env}_REDIRECT_URL) with the provider.",
        provider.setting_names().join(", "),
        env = provider.env
    ));
    notes.push(format!(
        "Try it offline: {} cargo run --features oauth-mock (development only).",
        provider.offline_settings()
    ));
    if let Some(note) = provider.note {
        notes.push(note.to_owned());
    }
    if first_provider {
        notes.push("Run `cargo rullst db:migrate` to create the oauth_identities table. A verified provider email that already belongs to an account is refused by default; see EXISTING_ACCOUNT_POLICY in src/controllers/oauth/mod.rs.".to_owned());
    }
    Ok(Plan { edits, notes })
}

/// Parses `source` and normalizes it with rustfmt.
fn rust(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    syn::parse_file(source)?;
    super::consumer_support::formatted(source)
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests;

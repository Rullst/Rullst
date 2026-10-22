//! `cargo rullst make:oauth <provider>`: sign-in with an OAuth/OpenID Connect
//! provider for the `make:auth`, SaaS and LMS account model (v13 preview).
//!
//! The generated code keeps login challenges in the application database
//! through `rullst::connect::challenge`, links provider identities in an
//! `oauth_identities` table and signs accounts in with the same session-epoch
//! cookie as the password login. Each provider is one [`ProviderSpec`]; add a
//! row (and its template and tests) to support another one.
use super::consumer_files::{self as writes, Edit};
use clap::{Arg, ArgMatches, Command};
use std::io;
use std::path::Path;

mod edits;
mod project;

/// A provider `make:oauth` can generate.
struct ProviderSpec {
    /// Route segment, cookie path, `oauth_identities.provider` and module name.
    name: &'static str,
    /// Human-readable name in the login link and messages.
    label: &'static str,
    /// Prefix of its settings: `<ENV>_CLIENT_ID`, `<ENV>_CLIENT_SECRET`,
    /// `<ENV>_REDIRECT_URL`.
    env: &'static str,
    /// The generated `src/controllers/oauth/<name>.rs`.
    template: &'static str,
}

const PROVIDERS: [ProviderSpec; 1] = [ProviderSpec {
    name: "google",
    label: "Google",
    env: "GOOGLE",
    template: include_str!("google.rs.template"),
}];

const CONTROLLER_TEMPLATE: &str = include_str!("controller.rs.template");
const IDENTITY_TEMPLATE: &str = include_str!("identity.rs.template");
const MIGRATION_TEMPLATE: &str = include_str!("migration.rs.template");
/// File-name suffix of the identities migration.
const MIGRATION_SUFFIX: &str = "_create_oauth_identities_table.rs";

pub(crate) fn command() -> Command {
    Command::new("make:oauth")
        .about("Add sign-in with an OAuth/OpenID Connect provider to the make:auth, SaaS or LMS accounts (v13 preview)")
        .arg(
            Arg::new("provider")
                .required(true)
                .value_parser(PROVIDERS.map(|provider| provider.name))
                .help("The identity provider; `google` for now"),
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
    let provider = PROVIDERS
        .iter()
        .find(|provider| provider.name == name)
        .ok_or_else(|| invalid("make:oauth supports only: google"))?;
    if timestamp.len() != 14 || !timestamp.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid("the migration timestamp must be 14 digits").into());
    }
    let root = &writes::project_root(root)?;
    let database = project::check(root)?;
    let mut edits = Vec::new();
    let mut notes = vec![format!(
        "{} sign-in added: GET /auth/{name} starts it and GET /auth/{name}/callback finishes it.",
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
        None => notes.push(format!(
            "Mount the routes yourself, behind the CSRF layer: call controllers::oauth::{name}::initialize()? where the router is built, then route GET /auth/{name} to controllers::oauth::{name}::start and GET /auth/{name}/callback to controllers::oauth::{name}::callback, each with controllers::auth_controller::credential_rate_limit."
        )),
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
        "Set {env}_CLIENT_ID and {env}_CLIENT_SECRET, and register <RULLST_PUBLIC_ORIGIN>/auth/{name}/callback (or {env}_REDIRECT_URL) with the provider.",
        env = provider.env
    ));
    notes.push(format!(
        "Try it offline: {env}_CLIENT_ID=mock_{name} {env}_CLIENT_SECRET=mock_{name} cargo run --features oauth-mock (development only).",
        env = provider.env
    ));
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

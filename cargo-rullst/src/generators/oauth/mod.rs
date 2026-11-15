//! `cargo rullst make:oauth <provider>`: sign-in with Google, GitHub,
//! Microsoft or Apple for the account model of `cargo rullst auth`, the SaaS
//! starter and the LMS example, or for an application's own sign-in through
//! its authentication contract (`src/auth_contract.rs`) (v13 preview).
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
use project::Accounts;
use providers::ProviderSpec;
use std::io;
use std::path::Path;

pub(crate) mod edits;
mod project;
mod providers;

const CONTROLLER_TEMPLATE: &str = include_str!("controller.rs.template");
const STARTER_HEADER: &str = include_str!("header_starter.rs.template");
const STARTER_ACCOUNTS: &str = include_str!("accounts_starter.rs.template");
const CONTRACT_HEADER: &str = include_str!("header_contract.rs.template");
const CONTRACT_ACCOUNTS: &str = include_str!("accounts_contract.rs.template");
const IDENTITY_TEMPLATE: &str = include_str!("identity.rs.template");
const IDENTITY_PASSWORD_TEMPLATE: &str = include_str!("identity_password.rs.template");
const MIGRATION_TEMPLATE: &str = include_str!("migration.rs.template");
/// Where the starter flow's `session_cookie` goes.
const SESSION_COOKIE_MARKER: &str = "__RULLST_SESSION_COOKIE__";
/// `session_cookie` of a starter that registers its sessions.
const REGISTERED_SESSION_COOKIE: &str = r#"/// The password login's 30-day `rullst_session` cookie for `user`: a new
/// registered session at its current session epoch
/// (`start_session` in src/middlewares/auth_middleware.rs), listed in "Your
/// devices" after the request's `User-Agent`. Both sign-in paths refuse an
/// account whose `current_session_epoch()` is `None`; if you add an account
/// status (for example blocked or inactive), check it here and in
/// `login_submit`.
pub(crate) async fn session_cookie(
    user: &User,
    headers: &HeaderMap,
) -> Result<String, rullst::auth::session_registry::SessionRegistryError> {
    crate::middlewares::auth_middleware::start_session(user, headers).await
}
"#;
/// `session_cookie` of a starter made before 13.0.0-alpha.6, which issues
/// stateless session-epoch cookies.
const EPOCH_SESSION_COOKIE: &str = r#"/// The password login's 30-day `rullst_session` cookie for `user`, carrying
/// its current session epoch. Both sign-in paths refuse an account whose
/// `current_session_epoch()` is `None`; if you add an account status (for
/// example blocked or inactive), check it here and in `login_submit`.
pub(crate) async fn session_cookie(
    user: &User,
    _headers: &HeaderMap,
) -> Result<String, rullst::auth::AuthError> {
    let epoch = user
        .current_session_epoch()
        .ok_or(rullst::auth::AuthError::SessionRevoked)?;
    rullst::auth::make_login_cookie_with_epoch(user.id, epoch, rullst::auth::DEFAULT_SESSION_MAX_AGE)
}
"#;
/// Where `make:mfa` puts its second step in the shared flow.
const SECOND_STEP_MARKER: &str = "    // __RULLST_MFA_SECOND_STEP__\n";
/// File-name suffix of the identities migration.
const MIGRATION_SUFFIX: &str = "_create_oauth_identities_table.rs";
const UNKNOWN_PROVIDER: &str = "make:oauth supports only: google, github, microsoft, apple";

/// The shared flow of `provider` for `database`, for the starter's
/// accounts (`registered` when the starter registers its sessions) or the
/// authentication contract, with the second step of `make:mfa` when the
/// project has it.
pub(crate) fn render_flow(
    provider: &str,
    database: &str,
    accounts: Accounts,
    registered: bool,
    second_step: bool,
) -> String {
    let (header, account_part) = match accounts {
        Accounts::Starter => (STARTER_HEADER, STARTER_ACCOUNTS),
        Accounts::Contract => (CONTRACT_HEADER, CONTRACT_ACCOUNTS),
    };
    let hook = if second_step {
        crate::generators::auth::mfa::oauth_hook_lines("    ")
    } else {
        String::new()
    };
    let session_cookie = if registered {
        REGISTERED_SESSION_COOKIE
    } else {
        EPOCH_SESSION_COOKIE
    };
    format!("{header}\n{CONTROLLER_TEMPLATE}\n{account_part}")
        .replace(SESSION_COOKIE_MARKER, session_cookie)
        .replace(SECOND_STEP_MARKER, &hook)
        .replace("__PROVIDER__", provider)
        .replace("__CHALLENGE_DATABASE__", database)
}

/// The shared flow for the starter's accounts.
#[cfg(test)]
pub(crate) fn render_controller(provider: &str, database: &str, second_step: bool) -> String {
    render_flow(provider, database, Accounts::Starter, true, second_step)
}

/// In a starter that registers its sessions, the password removal also
/// deletes the account's sessions, in the same transaction.
const REGISTERED_DOC: &str = " The same transaction deletes\n/// its registered sessions, so they also leave \"Your devices\".";
const REVOKE_REGISTERED_SESSIONS: &str = "    crate::middlewares::auth_middleware::Sessions::revoke_all_sessions_on(&mut **transaction, user_id)\n        .await\n        .map_err(|error| rullst::orm::Error::DatabaseError(error.to_string()))?;\n";

/// The identity model and migration for `accounts` (`registered` when the
/// starter registers its sessions).
fn render_identity(accounts: Accounts, registered: bool, migration: &str) -> (String, String) {
    let (doc, revoke) = if registered {
        (REGISTERED_DOC, REVOKE_REGISTERED_SESSIONS)
    } else {
        ("", "")
    };
    let password = IDENTITY_PASSWORD_TEMPLATE
        .replace("__RULLST_REGISTERED_DOC__", doc)
        .replace("__RULLST_REVOKE_REGISTERED_SESSIONS__", revoke);
    let (identity, imports, column) = match accounts {
        Accounts::Starter => (
            format!("{IDENTITY_TEMPLATE}{password}"),
            "ForeignKeyAction, Migration, Schema",
            "            table\n                .foreign_id(\"user_id\", \"users\")\n                .on_delete(ForeignKeyAction::Cascade);\n",
        ),
        Accounts::Contract => (
            IDENTITY_TEMPLATE.to_owned(),
            "Migration, Schema",
            "            // The account id of the application's own sign-in: delete its\n            // identities when you delete the account.\n            table.integer(\"user_id\").not_null();\n",
        ),
    };
    let migration = MIGRATION_TEMPLATE
        .replace("__MIGRATION_NAME__", migration)
        .replace("__SCHEMA_IMPORTS__", imports)
        .replace("__USER_ID_COLUMN__", column);
    (identity, migration)
}

pub(crate) fn command() -> Command {
    Command::new("make:oauth")
        .about("Add sign-in with an OAuth/OpenID Connect provider to the accounts of `cargo rullst auth`, the SaaS starter or the LMS example, or to an application's own sign-in through src/auth_contract.rs (v13 preview)")
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
    let (database, accounts) = project::check(root)?;
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
        let second_step = crate::generators::auth::mfa::has_second_step(root);
        let registered = accounts == Accounts::Starter
            && crate::generators::consumer_support::registers_sessions(root);
        let controller = render_flow(
            provider.name,
            database.sqlx_type(),
            accounts,
            registered,
            second_step,
        );
        edits.push(Edit::create(oauth_dir.join("mod.rs"), rust(&controller)?)?);
        if second_step {
            notes.push(
                "Accounts with two-step verification (make:mfa) finish this sign-in at /login/mfa."
                    .to_owned(),
            );
        }
        let migration = format!("m{timestamp}{}", MIGRATION_SUFFIX.trim_end_matches(".rs"));
        let (identity, source) = render_identity(accounts, registered, &migration);
        edits.push(Edit::create(
            root.join("src/models/oauth_identity.rs"),
            rust(&identity)?,
        )?);
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

    let limiter = match accounts {
        Accounts::Starter => "controllers::auth_controller::credential_rate_limit",
        Accounts::Contract => "controllers::oauth::credential_rate_limit",
    };
    let starter_router = match accounts {
        Accounts::Starter => edits::router(root, provider)?,
        Accounts::Contract => None,
    };
    match starter_router {
        Some(edit) => edits.push(edit),
        None if provider.form_post => notes.push(format!(
            "Mount the routes yourself, behind the CSRF layer: call controllers::oauth::{name}::initialize()? where the router is built, then route GET /auth/{name} to controllers::oauth::{name}::start, and POST /auth/{name}/callback to controllers::oauth::{name}::callback with GET to controllers::oauth::{name}::mock_callback, each with {limiter}."
        )),
        None => notes.push(format!(
            "Mount the routes yourself, behind the CSRF layer: call controllers::oauth::{name}::initialize()? where the router is built, then route GET /auth/{name} to controllers::oauth::{name}::start and GET /auth/{name}/callback to controllers::oauth::{name}::callback, each with {limiter}."
        )),
    }
    if provider.form_post
        && let Some(edit) = edits::csrf_exemption(root, &format!("/auth/{name}/callback"))?
    {
        edits.push(edit);
    }
    let login_page = match accounts {
        Accounts::Starter => edits::login_page(root, provider)?,
        Accounts::Contract => None,
    };
    match login_page {
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
        notes.push(match accounts {
            Accounts::Starter => "Run `cargo rullst db:migrate` to create the oauth_identities table. A verified provider email that already belongs to an account is refused by default; see EXISTING_ACCOUNT_POLICY in src/controllers/oauth/mod.rs.",
            Accounts::Contract => "Run `cargo rullst db:migrate` to create the oauth_identities table. New provider accounts come from auth_contract::create_account; a verified provider email that already belongs to an account is refused (src/controllers/oauth/mod.rs).",
        }.to_owned());
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
mod contract_tests;
#[cfg(test)]
mod tests;

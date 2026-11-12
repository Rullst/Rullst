// cargo-rullst/src/generators/auth/mfa.rs — `cargo rullst make:mfa`: two-step
// verification with pages, the second sign-in step and the reset command.

use super::{reject_turso_primary, require_migration_runner};
use crate::generators::auth_contract;
use crate::generators::consumer_files::{self as writes, Edit};
use crate::generators::is_rullst_project;
use std::error::Error;
use std::io;
use std::path::Path;

mod edits;
mod starter;

use edits::Changes;
pub(crate) use edits::{has_second_step, login_hook, oauth_hook_lines};

const CONTROLLER_TEMPLATE: &str = include_str!("mfa/controller.rs.template");
const FACTOR_TEMPLATE: &str = include_str!("mfa/factor.rs.template");
const CHALLENGE_TEMPLATE: &str = include_str!("mfa/challenge.rs.template");
const ADMIN_TEMPLATE: &str = include_str!("mfa/admin.rs.template");
const STARTER_ACCOUNT_TEMPLATE: &str = include_str!("mfa/account_starter.rs.template");
const CONTRACT_ACCOUNT_TEMPLATE: &str = include_str!("mfa/account_contract.rs.template");
const PAGES_TEMPLATE: &str = include_str!("mfa/pages.rs.template");
const MIGRATION_TEMPLATE: &str = include_str!("mfa/migration.rs.template");
/// File-name suffix of the migration; an existing one stops the run.
const MIGRATION_SUFFIX: &str = "_create_user_mfa_factors_table.rs";

/// Where the accounts come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Accounts {
    /// The `User` model and `auth_middleware` of `cargo rullst auth` or the
    /// SaaS starter.
    Starter,
    /// The application's own sign-in, through `src/auth_contract.rs`.
    Contract,
}

impl Accounts {
    /// The route middleware that inserts the account id.
    fn middleware(self) -> &'static str {
        match self {
            Self::Starter => "middlewares::auth_middleware::auth_middleware",
            Self::Contract => "auth_contract::require_account",
        }
    }

    fn account_template(self) -> &'static str {
        match self {
            Self::Starter => STARTER_ACCOUNT_TEMPLATE,
            Self::Contract => CONTRACT_ACCOUNT_TEMPLATE,
        }
    }

    /// The `rullst` features of the generated code: the starter adapter
    /// issues `rullst::auth` session cookies.
    fn features(self) -> &'static [&'static str] {
        match self {
            Self::Starter => &["orm", "security", "auth"],
            Self::Contract => &["orm", "security"],
        }
    }
}

/// The application files the scaffold writes, relative to the project; any
/// that exists stops the run.
const OUTPUTS: [&str; 6] = [
    "src/controllers/mfa.rs",
    "src/controllers/mfa/account.rs",
    "src/controllers/mfa/admin.rs",
    "src/controllers/mfa/challenge.rs",
    "src/controllers/mfa/factor.rs",
    "src/pages/mfa.rs",
];

/// Every generated source with its project path, for `accounts`.
pub(crate) fn render_sources(accounts: Accounts) -> [(&'static str, &'static str); 6] {
    [
        (OUTPUTS[0], CONTROLLER_TEMPLATE),
        (OUTPUTS[1], accounts.account_template()),
        (OUTPUTS[2], ADMIN_TEMPLATE),
        (OUTPUTS[3], CHALLENGE_TEMPLATE),
        (OUTPUTS[4], FACTOR_TEMPLATE),
        (OUTPUTS[5], PAGES_TEMPLATE),
    ]
}

/// Returns the reversible `user_mfa_factors` and `mfa_challenges` migration.
pub(crate) fn render_mfa_migration(migration_name: &str) -> String {
    MIGRATION_TEMPLATE.replace("__MIGRATION_NAME__", migration_name)
}

fn invalid(message: String) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidInput, message).into()
}

/// What `make:mfa` writes and changes, and what it tells the developer.
pub(crate) struct Plan {
    edits: Vec<Edit>,
    notes: Vec<String>,
    accounts: Accounts,
}

pub fn scaffold_mfa_system() -> Result<(), Box<dyn Error>> {
    if !is_rullst_project() {
        return Err(crate::ui::error_report::ProjectRequired.into());
    }
    reject_turso_primary("make:mfa")?;
    require_migration_runner("make:mfa")?;
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    let plan = plan(&std::env::current_dir()?, &timestamp)?;
    writes::apply(&plan.edits)?;
    println!(
        "🔐 Two-step verification added (TOTP, with pages and the second sign-in step), {}.",
        match plan.accounts {
            Accounts::Starter => "for the accounts of `cargo rullst auth`",
            Accounts::Contract => "for this application's sign-in (src/auth_contract.rs)",
        }
    );
    for note in plan.notes {
        println!("👉 {note}");
    }
    Ok(())
}

/// Which accounts the project has: the starter's when its model and
/// middleware are there, else the authentication contract.
fn accounts(root: &Path) -> Result<Accounts, Box<dyn Error>> {
    let missing = starter::missing(root)?;
    if missing.is_empty() {
        return Ok(Accounts::Starter);
    }
    if auth_contract::present(root)? {
        auth_contract::check(root, auth_contract::MFA)?;
        return Ok(Accounts::Contract);
    }
    Err(auth_contract::neither(
        auth_contract::MFA,
        &format!("this project lacks {}", missing.join(", ")),
    ))
}

pub(crate) fn plan(root: &Path, timestamp: &str) -> Result<Plan, Box<dyn Error>> {
    if timestamp.len() != 14 || !timestamp.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid(
            "the migration timestamp must be 14 digits".to_owned(),
        ));
    }
    // Links above the project are the developer's; links beneath it are refused.
    let root = &writes::project_root(root)?;
    let mut existing = Vec::new();
    for output in OUTPUTS {
        if std::fs::symlink_metadata(root.join(output)).is_ok() {
            existing.push(output.to_owned());
        }
    }
    existing.extend(edits::migrations_ending(root, MIGRATION_SUFFIX)?);
    if !existing.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "refusing to overwrite existing second-factor scaffold: {}; integrate the second factor into the existing files manually",
                existing.join(", ")
            ),
        )
        .into());
    }
    let accounts = accounts(root)?;

    let mut edits = Vec::new();
    for (path, source) in render_sources(accounts) {
        syn::parse_file(source)?;
        edits.push(Edit::create(root.join(path), source.to_owned())?);
    }
    let migration = format!("m{timestamp}{}", MIGRATION_SUFFIX.trim_end_matches(".rs"));
    let migrations = root.join("src/migrations");
    edits.push(Edit::create(
        migrations.join(format!("{migration}.rs")),
        render_mfa_migration(&migration),
    )?);

    let mut changes = Changes::default();
    let manifest = changes.current(&root.join("Cargo.toml"))?;
    *manifest = crate::generators::chat::ensure_rullst_features(manifest, accounts.features())?;
    edits::migrations_index(&mut changes, &migrations, &migration)?;
    changes.declare(&edits::module_file(root, "controllers")?, "mfa")?;
    changes.declare(&edits::module_file(root, "pages")?, "mfa")?;
    let crate_root = edits::crate_root(root)?;
    for module in ["controllers", "pages"] {
        changes.declare(&crate_root, module)?;
    }

    let mut notes = vec![
        "Run `cargo rullst db:migrate` to create the user_mfa_factors and mfa_challenges tables."
            .to_owned(),
    ];
    notes.extend(edits::wire(root, accounts, &mut changes)?);
    notes.push("Authenticator apps show the account's e-mail and the issuer MFA_ISSUER, else APP_NAME (environment or .env), else APPLICATION_NAME in src/controllers/mfa/factor.rs.".to_owned());
    notes.push("TOTP secrets are stored unencrypted in user_mfa_factors: protect that table and its backups like credentials. Recovery codes remain application work; `admin:mfa-reset <email>` handles a lost phone.".to_owned());
    edits.extend(changes.into_edits()?);
    Ok(Plan {
        edits,
        notes,
        accounts,
    })
}

#[cfg(test)]
mod tests;

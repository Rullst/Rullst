// cargo-rullst/src/generators/auth/users_table.rs — How `cargo rullst auth`
// treats an existing `users` table: create it, extend the Blank/API starter's
// minimal one, or refuse a table it does not recognize.

use crate::blueprints::blank::{STARTER_USER_MODEL, STARTER_USERS_MIGRATION};
use crate::generators::output_guard::{existing_migrations, write_output};
use quote::ToTokens;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::{Path, PathBuf};

/// File name suffix of the migration that adds the account columns to the
/// starter's `users` table.
pub(crate) const ACCOUNT_COLUMNS_SUFFIX: &str = "_add_account_columns_to_users_table.rs";

const ACCOUNT_COLUMNS_TEMPLATE: &str = include_str!("users_account_columns_migration.rs.template");

/// What `cargo rullst auth` does with the project's `users` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UsersTable {
    /// No migration creates `users`: write the complete table.
    Create,
    /// The unchanged starter migration at this path creates `users` with id,
    /// name and timestamps: add the account columns in a new migration.
    ExtendStarter(PathBuf),
}

/// Plans the `users` table from `src/migrations`, refusing a second
/// account-columns migration and any `users` migration other than the
/// unchanged Blank/API starter one.
pub(crate) fn plan() -> Result<UsersTable, IoError> {
    if let Some(existing) = existing_migrations(&[ACCOUNT_COLUMNS_SUFFIX.to_string()])?.first() {
        return Err(IoError::new(
            ErrorKind::AlreadyExists,
            format!(
                "refusing to add the account columns twice: {} already adds them to users",
                existing.display()
            ),
        ));
    }
    let users = existing_migrations(&[
        "_create_users.rs".to_string(),
        "_create_users_table.rs".to_string(),
    ])?;
    if users.is_empty() {
        return Ok(UsersTable::Create);
    }
    if let [only] = users.as_slice()
        && is_starter_users_migration(&fs::read_to_string(only)?)
    {
        return Ok(UsersTable::ExtendStarter(only.clone()));
    }
    let users = users
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    Err(IoError::new(
        ErrorKind::AlreadyExists,
        format!(
            "refusing to change the users table: {} already creates it, and only the unchanged users migration of the Blank and API starters (id, name and timestamps) can be extended. Add email (VARCHAR(255) NOT NULL with the unique index users_email_unique), password_hash, oauth_provider, oauth_id, is_admin and session_epoch with `cargo rullst make:migration` and integrate authentication manually",
            users.join(", ")
        ),
    ))
}

/// Whether `source` is the starter's `users` migration, ignoring formatting
/// and comments.
pub(crate) fn is_starter_users_migration(source: &str) -> bool {
    match (tokens(source), tokens(STARTER_USERS_MIGRATION)) {
        (Some(source), Some(starter)) => source == starter,
        _ => false,
    }
}

fn tokens(source: &str) -> Option<String> {
    syn::parse_file(source)
        .ok()
        .map(|file| file.into_token_stream().to_string())
}

/// The migration named `migration_name` that adds the account columns to
/// the starter's `users` table.
pub(crate) fn render_account_columns_migration(migration_name: &str) -> String {
    ACCOUNT_COLUMNS_TEMPLATE.replace("__MIGRATION_NAME__", migration_name)
}

/// What replaces the starter's `User` model in the crate root.
const STARTER_MODEL_REPLACEMENT: &str = "// The account model of `cargo rullst auth`, which owns the users table.\npub use crate::models::user::User;\n";

/// Replaces the starter's two-field `User` model in `root` with a re-export
/// of the account model, so the project has one `User` for `users`. Returns
/// whether the model was found unchanged (as generated, or with its imports
/// sorted by rustfmt) and replaced.
pub(crate) fn replace_starter_user_model(root: &Path) -> Result<bool, IoError> {
    let source = fs::read_to_string(root)?;
    let sorted = STARTER_USER_MODEL.replacen(
        "{Orm, FromRow, RowSnapshot}",
        "{FromRow, Orm, RowSnapshot}",
        1,
    );
    for starter in [STARTER_USER_MODEL, sorted.as_str()] {
        if source.matches(starter).count() == 1 {
            let replaced = source.replacen(starter, STARTER_MODEL_REPLACEMENT, 1);
            write_output(root, replaced.as_bytes(), true)?;
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
#[path = "users_table_tests.rs"]
mod tests;

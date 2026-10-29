//! What `make:oauth` requires of the project before it writes anything.
use super::super::consumer_files as writes;
use super::invalid;
use std::fs;
use std::io;
use std::path::Path;
use syn::{Item, Visibility};

/// The project's single SQL database, from its strict ORM profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Database {
    Sqlite,
    Postgres,
    MySql,
}

impl Database {
    /// The SQLx database type of the application's pool.
    pub(crate) fn sqlx_type(self) -> &'static str {
        match self {
            Self::Sqlite => "rullst::db::sqlx::Sqlite",
            Self::Postgres => "rullst::db::sqlx::Postgres",
            Self::MySql => "rullst::db::sqlx::MySql",
        }
    }

    /// The `rullst` feature with this database's challenge store.
    pub(crate) fn feature(self) -> &'static str {
        match self {
            Self::Sqlite => "oauth-sqlite",
            Self::Postgres => "oauth-postgres",
            Self::MySql => "oauth-mysql",
        }
    }
}

const ACCOUNT_MODEL: &str = "make:oauth needs the account model of `cargo rullst auth` or of the SaaS starter (src/models/user.rs with `session_epoch`, `current_session_epoch` and `find_by_email`), and its auth_controller; this project has none";

/// Checks the account model, the auth helpers, the login page, the migration
/// runner and the database; returns the database.
pub(crate) fn check(root: &Path) -> Result<Database, Box<dyn std::error::Error>> {
    let manifest = writes::read(&root.join("Cargo.toml"))
        .map_err(|_| invalid("run make:oauth in a Rullst project directory"))?;
    let database = database(&manifest)?;
    let user = parsed(root, "src/models/user.rs")?;
    let fields_and_methods = [
        "session_epoch",
        "oauth_provider",
        "oauth_id",
        "email",
        "current_session_epoch",
        "find_by_email",
    ];
    if !fields_and_methods.iter().all(|name| declares(&user, name)) {
        return Err(invalid(ACCOUNT_MODEL).into());
    }
    let controller = parsed(root, "src/controllers/auth_controller.rs")?;
    for helper in [
        "get_csrf_token",
        "get_csp_nonce",
        "normalize_email",
        "valid_email",
        "utc_timestamp",
        "credential_rate_limit",
    ] {
        if !public_function(&controller, helper) {
            return Err(invalid(ACCOUNT_MODEL).into());
        }
    }
    if !public_function(&parsed(root, "src/pages/auth.rs")?, "login_page") {
        return Err(invalid(ACCOUNT_MODEL).into());
    }
    if !migration_runner(root)? {
        return Err(invalid("make:oauth adds a SQL migration, but this project has no migration runner: declare `pub mod migrations;` in src/main.rs or src/lib.rs and call `rullst::artisan!(crate::migrations::get_migrations());` at startup").into());
    }
    Ok(database)
}

/// The strict profile of the `rullst` (or `rullst-orm`) dependency.
pub(crate) fn database(manifest: &str) -> Result<Database, Box<dyn std::error::Error>> {
    let manifest: toml::Table = toml::from_str(manifest)?;
    let dependencies = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| invalid("Cargo.toml has no [dependencies]"))?;
    if !dependencies.contains_key("rullst") {
        return Err(invalid("run make:oauth in a Rullst project directory").into());
    }
    let mut selected = Vec::new();
    for name in ["rullst", "rullst-orm"] {
        let features = dependencies
            .get(name)
            .and_then(toml::Value::as_table)
            .and_then(|dependency| dependency.get("features"))
            .and_then(toml::Value::as_array);
        for feature in features
            .into_iter()
            .flatten()
            .filter_map(toml::Value::as_str)
        {
            let database = match feature {
                "orm-turso" | "turso" => {
                    return Err(invalid("make:oauth generates a SQLx account store; Turso-primary projects are not supported").into());
                }
                "strict-sqlite" => Database::Sqlite,
                "strict-postgres" => Database::Postgres,
                "strict-mysql" => Database::MySql,
                _ => continue,
            };
            if !selected.contains(&database) {
                selected.push(database);
            }
        }
    }
    match selected.as_slice() {
        [database] => Ok(*database),
        _ => Err(invalid("make:oauth stores login challenges in the application database and needs exactly one strict database profile (strict-sqlite, strict-postgres or strict-mysql), as `cargo rullst new --database ...` selects").into()),
    }
}

fn parsed(root: &Path, relative: &str) -> Result<syn::File, Box<dyn std::error::Error>> {
    let source = writes::read(&root.join(relative)).map_err(|_| invalid(ACCOUNT_MODEL))?;
    Ok(syn::parse_file(&source).map_err(|_| invalid(ACCOUNT_MODEL))?)
}

/// Whether the `User` struct has the field `name` or an impl of it the method.
fn declares(file: &syn::File, name: &str) -> bool {
    file.items.iter().any(|item| match item {
        Item::Struct(user) if user.ident == "User" => user
            .fields
            .iter()
            .any(|field| field.ident.as_ref().is_some_and(|ident| ident == name)),
        Item::Impl(block) => {
            matches!(&*block.self_ty, syn::Type::Path(path) if path.path.is_ident("User"))
                && block.items.iter().any(
                    |item| matches!(item, syn::ImplItem::Fn(method) if method.sig.ident == name),
                )
        }
        _ => false,
    })
}

/// Whether a top-level function `name` is visible outside its module.
fn public_function(file: &syn::File, name: &str) -> bool {
    file.items.iter().any(|item| {
        matches!(item, Item::Fn(function)
            if function.sig.ident == name && !matches!(function.vis, Visibility::Inherited))
    })
}

fn migration_runner(root: &Path) -> io::Result<bool> {
    let mut declared = false;
    let mut called = false;
    for relative in ["src/main.rs", "src/lib.rs"] {
        let Ok(source) = writes::read(&root.join(relative)) else {
            continue;
        };
        declared |= syn::parse_file(&source).is_ok_and(|file| {
            file.items
                .iter()
                .any(|item| matches!(item, Item::Mod(module) if module.ident == "migrations"))
        });
        called |= source.contains("get_migrations(");
    }
    Ok(declared && called && root.join("src/migrations/mod.rs").is_file())
}

/// Whether `src/migrations` holds a file ending in `suffix`.
pub(crate) fn has_migration(root: &Path, suffix: &str) -> io::Result<bool> {
    let entries = match fs::read_dir(root.join("src/migrations")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    for entry in entries {
        if entry?
            .file_name()
            .to_str()
            .is_some_and(|name| name.ends_with(suffix))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

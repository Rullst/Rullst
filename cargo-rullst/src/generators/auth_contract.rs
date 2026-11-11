//! The authentication contract: how `make:mfa`, `make:oauth` and
//! `make:privacy` reach an application's own sign-in.
//!
//! A project without the account files of `cargo rullst auth` (or the SaaS
//! starter) writes `src/auth_contract.rs` with the functions below and
//! declares `mod auth_contract;` in its crate root. Each generator checks the
//! functions it calls, by name and `async`, before it writes anything; the
//! compiler checks the signatures the CLI reference documents.
use super::consumer_files as writes;
use std::error::Error;
use std::io;
use std::path::Path;
use syn::{Fields, Item, Visibility};

/// Where the contract lives, relative to the project root.
pub(crate) const PATH: &str = "src/auth_contract.rs";
/// The reference documentation of the contract.
pub(crate) const DOCS: &str =
    "https://rullst.github.io/Rullst/book/cli_reference.html#authentication-contract";
/// The account type of the contract.
const ACCOUNT: &str = "ContractAccount";

/// One function of the contract: its name and whether it is `async`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Function {
    name: &'static str,
    asynchronous: bool,
}

impl Function {
    const fn new(name: &'static str, asynchronous: bool) -> Self {
        Self { name, asynchronous }
    }

    /// How the refusal names it: `pub async fn name` or `pub fn name`.
    fn signature(self) -> String {
        let asynchronous = if self.asynchronous { "async " } else { "" };
        format!("`pub {asynchronous}fn {}`", self.name)
    }
}

/// Route middleware that inserts the signed-in account's id as `Extension<i32>`.
const REQUIRE_ACCOUNT: Function = Function::new("require_account", true);
/// The account with an id.
const FIND_ACCOUNT: Function = Function::new("find_account", true);
/// The account with a (trimmed, lowercase) e-mail address.
const FIND_ACCOUNT_BY_EMAIL: Function = Function::new("find_account_by_email", true);
/// The `Set-Cookie` value of a full session for an account.
const SIGN_IN_COOKIE: Function = Function::new("sign_in_cookie", true);
/// Ends every session of an account.
const END_SESSIONS: Function = Function::new("end_sessions", true);
/// An opaque value naming the browser's current sign-in.
const SESSION_BINDING: Function = Function::new("session_binding", false);
/// Creates a passwordless account inside the caller's transaction.
const CREATE_ACCOUNT: Function = Function::new("create_account", true);

/// What one generator needs from the contract.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Needs {
    command: &'static str,
    functions: &'static [Function],
    fields: &'static [&'static str],
}

/// `make:mfa`: the signed-in account, its e-mail (the authenticator label),
/// lookups by id and e-mail (the reset command), sessions after the second
/// step and ending the others when the factor is turned on.
pub(crate) const MFA: Needs = Needs {
    command: "make:mfa",
    functions: &[
        REQUIRE_ACCOUNT,
        FIND_ACCOUNT,
        FIND_ACCOUNT_BY_EMAIL,
        SIGN_IN_COOKIE,
        END_SESSIONS,
    ],
    fields: &["id", "email"],
};

/// `make:privacy`: the signed-in account, its profile for the export and
/// the sign-in its forms are bound to.
pub(crate) const PRIVACY: Needs = Needs {
    command: "make:privacy",
    functions: &[REQUIRE_ACCOUNT, FIND_ACCOUNT, SESSION_BINDING],
    fields: &["id", "email", "name"],
};

/// `make:oauth`: accounts by id and verified e-mail, new passwordless
/// accounts and the session of a provider sign-in.
pub(crate) const OAUTH: Needs = Needs {
    command: "make:oauth",
    functions: &[
        FIND_ACCOUNT,
        FIND_ACCOUNT_BY_EMAIL,
        CREATE_ACCOUNT,
        SIGN_IN_COOKIE,
    ],
    fields: &["id", "email"],
};

fn invalid(message: String) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidInput, message).into()
}

/// Whether the project has `src/auth_contract.rs`.
pub(crate) fn present(root: &Path) -> io::Result<bool> {
    root.join(PATH).try_exists()
}

/// The refusal of a project that has neither the starter's accounts (the
/// `starter` part names what is missing) nor the contract.
pub(crate) fn neither(needs: Needs, starter: &str) -> Box<dyn Error> {
    invalid(format!(
        "{} needs the accounts of `cargo rullst auth` or the SaaS starter ({starter}), or, for an application with its own sign-in, the authentication contract in {PATH} ({DOCS})",
        needs.command
    ))
}

/// Checks that `src/auth_contract.rs` declares what `needs` calls and that
/// every crate root compiling the controllers declares the module.
pub(crate) fn check(root: &Path, needs: Needs) -> Result<(), Box<dyn Error>> {
    let command = needs.command;
    let source = writes::read(&root.join(PATH))
        .map_err(|error| invalid(format!("{command} could not read {PATH}: {error}")))?;
    let file = syn::parse_file(&source)
        .map_err(|_| invalid(format!("{command}: {PATH} is not valid Rust")))?;
    let mut missing = needs
        .functions
        .iter()
        .filter(|function| !declares(&file, **function))
        .map(|function| function.signature())
        .collect::<Vec<_>>();
    let fields = account_fields(&file);
    match fields {
        Some(fields) => missing.extend(
            needs
                .fields
                .iter()
                .filter(|field| !fields.iter().any(|declared| declared == *field))
                .map(|field| format!("the `pub {field}` field of `{ACCOUNT}`")),
        ),
        None => missing.push(format!("`pub struct {ACCOUNT}`")),
    }
    if !missing.is_empty() {
        return Err(invalid(format!(
            "{command} uses this application's own sign-in through {PATH}, which lacks {} ({DOCS})",
            missing.join(", ")
        )));
    }
    for crate_root in crate_roots(root)? {
        let source = writes::read(&root.join(crate_root))?;
        let declared =
            syn::parse_file(&source).is_ok_and(|file| declares_module(&file, "auth_contract"));
        if !declared {
            return Err(invalid(format!(
                "{command}: {crate_root} compiles the controllers but does not declare `mod auth_contract;` ({PATH})"
            )));
        }
    }
    Ok(())
}

/// Whether `file` has a public top-level function with the name and
/// `async`-ness of `function`.
fn declares(file: &syn::File, function: Function) -> bool {
    file.items.iter().any(|item| {
        matches!(item, Item::Fn(declared)
            if declared.sig.ident == function.name
                && matches!(declared.vis, Visibility::Public(_))
                && declared.sig.asyncness.is_some() == function.asynchronous)
    })
}

/// The public named fields of `pub struct ContractAccount`, if declared.
fn account_fields(file: &syn::File) -> Option<Vec<String>> {
    file.items.iter().find_map(|item| match item {
        Item::Struct(account)
            if account.ident == ACCOUNT && matches!(account.vis, Visibility::Public(_)) =>
        {
            let Fields::Named(fields) = &account.fields else {
                return Some(Vec::new());
            };
            Some(
                fields
                    .named
                    .iter()
                    .filter(|field| matches!(field.vis, Visibility::Public(_)))
                    .filter_map(|field| field.ident.as_ref().map(ToString::to_string))
                    .collect(),
            )
        }
        _ => None,
    })
}

fn declares_module(file: &syn::File, name: &str) -> bool {
    file.items
        .iter()
        .any(|item| matches!(item, Item::Mod(module) if module.ident == name))
}

/// The crate roots that compile the controllers: `src/lib.rs` and
/// `src/main.rs` when they declare `mod controllers` (the hot-reload
/// starters declare it in both), else the crate root.
fn crate_roots(root: &Path) -> io::Result<Vec<&'static str>> {
    let mut roots = Vec::new();
    let mut existing = Vec::new();
    for crate_root in ["src/lib.rs", "src/main.rs"] {
        let path = root.join(crate_root);
        if !path.try_exists()? {
            continue;
        }
        existing.push(crate_root);
        let source = writes::read(&path)?;
        if syn::parse_file(&source).is_ok_and(|file| declares_module(&file, "controllers")) {
            roots.push(crate_root);
        }
    }
    if roots.is_empty() {
        roots.extend(existing.first());
    }
    Ok(roots)
}

/// Turns the generated SaaS starter at `root` into an application with its
/// own sign-in, for the generator tests: the account model moves to
/// `src/models/account.rs`, so no generator recognizes the starter's
/// accounts, and the reference contract reaches it.
#[cfg(test)]
pub(crate) fn own_sign_in(root: &Path) {
    use std::fs;
    fs::rename(
        root.join("src/models/user.rs"),
        root.join("src/models/account.rs"),
    )
    .unwrap();
    let mut pending = vec![root.join("src")];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let source = fs::read_to_string(&path).unwrap();
                let moved = source
                    .replace("models::user::", "models::account::")
                    .replace("pub mod user;", "pub mod account;");
                fs::write(&path, moved).unwrap();
            }
        }
    }
    let contract = include_str!("auth_contract/reference.rs.template")
        .replace("crate::models::user::User", "crate::models::account::User");
    fs::write(root.join(PATH), contract).unwrap();
    for crate_root in ["src/lib.rs", "src/main.rs"] {
        let path = root.join(crate_root);
        if let Ok(source) = fs::read_to_string(&path)
            && source.contains("pub mod controllers;")
        {
            let declared = source.replacen(
                "pub mod controllers;",
                "pub mod auth_contract;\npub mod controllers;",
                1,
            );
            fs::write(&path, declared).unwrap();
        }
    }
}

#[cfg(test)]
mod tests;

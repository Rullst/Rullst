//! The changes `make:mfa` makes to existing project files: module
//! declarations, the migration index, and the wiring of the sign-in
//! handlers, the starter router, the dashboard and `main`.
use super::Accounts;
use crate::generators::consumer_files::{self as writes, Edit};
use crate::generators::{ProjectOrmBackend, insert_module_declaration, is_valid_rust_identifier};
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::{Item, ItemFn};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Planned changes to existing files: every change to one file makes one
/// edit, checked against the file as it was read.
#[derive(Default)]
pub(super) struct Changes {
    files: Vec<(PathBuf, Option<String>, String)>,
}

impl Changes {
    /// The planned content of `path`: as read (empty when it does not exist
    /// yet), with the changes planned so far.
    pub(super) fn current(&mut self, path: &Path) -> io::Result<&mut String> {
        let index = match self.files.iter().position(|(known, _, _)| known == path) {
            Some(index) => index,
            None => {
                let original = match writes::read(path) {
                    Ok(source) => Some(source),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                    Err(error) => return Err(error),
                };
                let updated = original.clone().unwrap_or_default();
                self.files.push((path.to_path_buf(), original, updated));
                self.files.len() - 1
            }
        };
        self.files
            .get_mut(index)
            .map(|(_, _, updated)| updated)
            .ok_or_else(|| io::Error::other("planned file vanished"))
    }

    /// Applies `change` to the planned content of `path`; `None` keeps it.
    fn change(
        &mut self,
        path: &Path,
        change: impl FnOnce(&str) -> Result<Option<String>>,
    ) -> Result<bool> {
        let source = self.current(path)?;
        match change(source)? {
            Some(updated) => {
                *source = updated;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Declares `pub mod <module>;` in `path` unless it declares the module.
    pub(super) fn declare(&mut self, path: &Path, module: &str) -> Result<()> {
        self.change(path, |source| {
            let declared = syn::parse_file(source)?
                .items
                .iter()
                .any(|item| matches!(item, Item::Mod(declared) if declared.ident == module));
            Ok((!declared)
                .then(|| insert_module_declaration(source, module, &format!("pub mod {module};"))))
        })?;
        Ok(())
    }

    pub(super) fn into_edits(self) -> io::Result<Vec<Edit>> {
        let mut edits = Vec::new();
        for (path, original, updated) in self.files {
            match original {
                Some(original) if original != updated => {
                    edits.push(Edit::replace(path, original, updated));
                }
                Some(_) => {}
                None => edits.push(Edit::create(path, updated)?),
            }
        }
        Ok(edits)
    }
}

/// The crate root: `src/lib.rs` when it exists, else `src/main.rs`.
pub(super) fn crate_root(root: &Path) -> io::Result<PathBuf> {
    let library = root.join("src/lib.rs");
    if library.try_exists()? {
        return Ok(library);
    }
    Ok(root.join("src/main.rs"))
}

/// The file of module `name` under `src/`: `src/<name>/mod.rs`, or
/// `src/<name>.rs` when the project uses that layout.
pub(super) fn module_file(root: &Path, name: &str) -> io::Result<PathBuf> {
    let flat = root.join(format!("src/{name}.rs"));
    if flat.try_exists()? {
        return Ok(flat);
    }
    Ok(root.join(format!("src/{name}/mod.rs")))
}

/// The `src/migrations` files ending in `suffix`, as project paths.
pub(super) fn migrations_ending(root: &Path, suffix: &str) -> io::Result<Vec<String>> {
    let entries = match fs::read_dir(root.join("src/migrations")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut found = Vec::new();
    for entry in entries {
        if let Some(name) = entry?.file_name().to_str()
            && name.ends_with(suffix)
        {
            found.push(format!("src/migrations/{name}"));
        }
    }
    found.sort();
    Ok(found)
}

/// `src/migrations/mod.rs` listing every migration plus `added`; the
/// project's own items there (seed modules, helpers) are kept.
pub(super) fn migrations_index(changes: &mut Changes, directory: &Path, added: &str) -> Result<()> {
    let mut modules = vec![added.to_owned()];
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries.collect::<io::Result<Vec<_>>>()?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "rs")
            && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
            && stem != "mod"
            && stem.starts_with('m')
        {
            if !is_valid_rust_identifier(stem) {
                return Err(super::invalid(format!(
                    "src/migrations/{stem}.rs is not a valid migration module name; rename it to ASCII letters, digits and '_'"
                )));
            }
            modules.push(stem.to_owned());
        }
    }
    modules.sort();
    modules.dedup();
    changes.change(&directory.join("mod.rs"), |source| {
        let existing = (!source.is_empty()).then_some(source);
        Ok(Some(crate::generators::migration::render_migrations_index(
            existing,
            &modules,
            ProjectOrmBackend::Sqlx,
        )?))
    })?;
    Ok(())
}

/// The top-level function `name` of `file`.
fn function<'a>(file: &'a syn::File, name: &str) -> Option<&'a ItemFn> {
    file.items.iter().find_map(|item| match item {
        Item::Fn(function) if function.sig.ident == name => Some(function),
        _ => None,
    })
}

/// `source` with `lines` (each indented like the line of `anchor`) inserted
/// before the line of the one `anchor` inside function `name`; `None` when
/// the function, a single anchor or `required` is missing, or `skip` is
/// already there.
fn insert_before(
    source: &str,
    name: &str,
    anchor: &str,
    required: &[&str],
    skip: &str,
    lines: &[&str],
) -> Result<Option<String>> {
    let file = syn::parse_file(source)?;
    let Some(range) = function(&file, name).map(|function| function.span().byte_range()) else {
        return Ok(None);
    };
    let Some(body) = source.get(range.clone()) else {
        return Ok(None);
    };
    if body.contains(skip)
        || body.matches(anchor).count() != 1
        || required.iter().any(|needed| !body.contains(needed))
    {
        return Ok(None);
    }
    let Some(at) = body.find(anchor).map(|at| range.start + at) else {
        return Ok(None);
    };
    let line_start = source.get(..at).map_or(0, |before| {
        before.rfind('\n').map_or(0, |newline| newline + 1)
    });
    let indent = source.get(line_start..at).unwrap_or_default();
    if !indent
        .chars()
        .all(|character| character == ' ' || character == '\t')
    {
        return Ok(None);
    }
    let block = indented(lines, indent);
    let mut updated = source.to_owned();
    updated.insert_str(line_start, &block);
    syn::parse_file(&updated)?;
    Ok(Some(updated))
}

/// The second step in the password sign-in of `cargo rullst auth`.
const LOGIN_HOOK: [&str; 10] = [
    "// Two-step verification (cargo rullst make:mfa): an account with a",
    "// second factor gets a five-minute challenge at /login/mfa instead.",
    "match crate::controllers::mfa::second_step(user.id, \"/dashboard\").await {",
    "    Ok(Some(second_step)) => return second_step,",
    "    Ok(None) => {}",
    "    Err(error) => {",
    "        return auth::login_page(&token, Some(error.message()), None, nonce).into_response();",
    "    }",
    "}",
    "",
];

/// `login_submit` of `src/controllers/auth_controller.rs` with the second
/// step before its session; `None` when it is not recognized.
pub(super) fn login_hook(source: &str) -> Result<Option<String>> {
    insert_before(
        source,
        "login_submit",
        "match login_cookie(&user) {",
        &["auth::login_page(&token", "nonce"],
        "mfa::second_step",
        &LOGIN_HOOK,
    )
}

/// The second step after a provider sign-in of `make:oauth`, which also
/// clears the provider's challenge cookie.
pub(crate) const OAUTH_HOOK: [&str; 18] = [
    "// Two-step verification (cargo rullst make:mfa): an account with a",
    "// second factor finishes signing in at /login/mfa.",
    "let next = return_to.as_ref().map_or(DEFAULT_RETURN_PATH, ReturnPath::as_str);",
    "match crate::controllers::mfa::second_step(user.id, next).await {",
    "    Ok(Some(mut second_step)) => {",
    "        if let Ok(value) = challenge_cookie(provider_name).and_then(|cookie| {",
    "            HeaderValue::from_str(&cookie.clear()).map_err(|_| ChallengeError::Invalid(\"cookie\"))",
    "        }) {",
    "            second_step.headers_mut().append(header::SET_COOKIE, value);",
    "        }",
    "        return second_step;",
    "    }",
    "    Ok(None) => {}",
    "    Err(error) => {",
    "        return refused(page, provider_name, StatusCode::FORBIDDEN, error.message());",
    "    }",
    "}",
    "",
];

/// `complete` of `src/controllers/oauth/mod.rs` with the second step before
/// its session; `None` when it is not recognized.
pub(super) fn oauth_hook(source: &str) -> Result<Option<String>> {
    insert_before(
        source,
        "complete",
        "let session = match session_cookie(&user)",
        &[
            "return_to",
            "DEFAULT_RETURN_PATH",
            "refused(page, provider_name",
        ],
        "mfa::second_step",
        &OAUTH_HOOK,
    )
}

/// The second step as `make:oauth` writes it into a new shared flow, for a
/// project that already has it: `indent` precedes each line.
pub(crate) fn oauth_hook_lines(indent: &str) -> String {
    indented(&OAUTH_HOOK, indent)
}

/// `lines` with `indent` before each one that is not empty.
fn indented(lines: &[&str], indent: &str) -> String {
    lines
        .iter()
        .map(|line| {
            if line.is_empty() {
                "\n".to_owned()
            } else {
                format!("{indent}{line}\n")
            }
        })
        .collect()
}

/// Whether the project has the `second_step` hook of this `make:mfa`.
pub(crate) fn has_second_step(root: &Path) -> bool {
    writes::read(&root.join("src/controllers/mfa.rs"))
        .is_ok_and(|source| source.contains("pub use challenge::second_step;"))
}

/// The routes of the starter router, after its password login route.
const ROUTES: &str = "
    // Two-step verification (cargo rullst make:mfa): the second sign-in step,
    // then the signed-in account's security page.
    .route(\"/login/mfa\", rullst::routing::get(controllers::mfa::challenge_view))
    .route(\"/login/mfa\", rullst::routing::post(controllers::mfa::challenge_submit)
        .layer(rullst::server::from_fn(controllers::auth_controller::credential_rate_limit)))
    .route(\"/account/security\", rullst::routing::get(controllers::mfa::security_view)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .route(\"/account/security/mfa/setup\", rullst::routing::post(controllers::mfa::setup)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .route(\"/account/security/mfa/confirm\", rullst::routing::post(controllers::mfa::confirm)
        .layer(rullst::server::from_fn(controllers::auth_controller::credential_rate_limit))
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
    .route(\"/account/security/mfa/disable\", rullst::routing::post(controllers::mfa::disable)
        .layer(rullst::server::from_fn(controllers::auth_controller::credential_rate_limit))
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))";

/// The starter router with the routes after its password login route;
/// `None` when it is not recognized or already has them.
pub(super) fn router_routes(source: &str) -> Result<Option<String>> {
    if source.contains("\"/login/mfa\"")
        || !source.contains("fn router(")
        || !source.contains("middlewares::auth_middleware::auth_middleware")
    {
        return Ok(None);
    }
    let file = syn::parse_file(source)?;
    let ends = crate::generators::oauth::edits::login_route_ends(&file)?;
    let [end] = ends.as_slice() else {
        return Ok(None);
    };
    let mut updated = source.to_owned();
    if !updated.is_char_boundary(*end) {
        return Ok(None);
    }
    updated.insert_str(*end, ROUTES);
    syn::parse_file(&updated)?;
    Ok(Some(updated))
}

/// The routes to mount by hand, with the middleware of `accounts`.
fn routes_note(accounts: Accounts) -> String {
    let middleware = accounts.middleware();
    format!(
        "Mount the routes inside your CSRF layer: GET /account/security → controllers::mfa::security_view and POST /account/security/mfa/setup, /account/security/mfa/confirm, /account/security/mfa/disable → controllers::mfa::{{setup, confirm, disable}}, each behind {middleware}; GET and POST /login/mfa → controllers::mfa::{{challenge_view, challenge_submit}} without it. Put your credential rate limit on the POST routes of /login/mfa, confirm and disable."
    )
}

const DASHBOARD_ANCHOR: &str =
    "<a href=\"/password/change\" class=\"btn-nexus\">🔑 Change password</a>";

/// The SaaS dashboard with a link to the security page next to "Change
/// password"; `None` when it is not recognized or already links it.
pub(super) fn dashboard_link(source: &str) -> Option<String> {
    if source.contains("\"/account/security\"") || source.matches(DASHBOARD_ANCHOR).count() != 1 {
        return None;
    }
    let at = source.find(DASHBOARD_ANCHOR)?;
    let line_start = source
        .get(..at)?
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    let indent = source.get(line_start..at)?;
    let mut updated = source.to_owned();
    updated.insert_str(
        at + DASHBOARD_ANCHOR.len(),
        &format!("\n{indent}<a href=\"/account/security\" class=\"btn-nexus\">🔐 Two-step verification</a>"),
    );
    Some(updated)
}

/// The admin command, before `rullst::artisan!` in an async `main`.
const MAIN_HOOK: [&str; 5] = [
    "// `<application> admin:mfa-reset <email>` (cargo rullst make:mfa) removes an",
    "// account's second factor and signs out its sessions, then exits.",
    "if let Some(status) = controllers::mfa::admin::run().await {",
    "    std::process::exit(status);",
    "}",
];

/// `src/main.rs` running the admin command first; `None` when `main` is not
/// async, the file does not declare `controllers` or has no single
/// `rullst::artisan!` call in `main`.
pub(super) fn main_hook(source: &str) -> Result<Option<String>> {
    let file = syn::parse_file(source)?;
    let declares_controllers = file
        .items
        .iter()
        .any(|item| matches!(item, Item::Mod(module) if module.ident == "controllers"));
    let asynchronous = function(&file, "main").is_some_and(|main| main.sig.asyncness.is_some());
    if !declares_controllers || !asynchronous {
        return Ok(None);
    }
    insert_before(
        source,
        "main",
        "rullst::artisan!(",
        &[],
        "mfa::admin::run()",
        &MAIN_HOOK,
    )
}

/// Wires the second factor into the sign-in handlers, the router, the
/// dashboard and `main` where they are recognized; returns a note for each
/// part done and each part left to the developer.
pub(super) fn wire(root: &Path, accounts: Accounts, changes: &mut Changes) -> Result<Vec<String>> {
    let mut notes = Vec::new();
    let login = root.join("src/controllers/auth_controller.rs");
    let starter_login = accounts == Accounts::Starter && login.try_exists()?;
    if starter_login && changes.change(&login, login_hook)? {
        notes.push("The password sign-in (login_submit) now sends accounts with a second factor to /login/mfa.".to_owned());
    } else {
        notes.push("In your sign-in handler, right after the password check, call `crate::controllers::mfa::second_step(account_id, \"<page after sign-in>\").await`: return its `Ok(Some(response))` instead of a session, and show `error.message()` for an `Err`.".to_owned());
    }
    let oauth = root.join("src/controllers/oauth/mod.rs");
    if oauth.try_exists()? {
        if changes.change(&oauth, oauth_hook)? {
            notes.push("Provider sign-in (make:oauth) also asks for the code: src/controllers/oauth/mod.rs calls second_step.".to_owned());
        } else {
            notes.push("Call second_step in src/controllers/oauth/mod.rs too, before the provider sign-in issues its session.".to_owned());
        }
    }
    let mut mounted = false;
    if accounts == Accounts::Starter {
        for router in ["src/lib.rs", "src/main.rs"].map(|path| root.join(path)) {
            if !mounted && router.try_exists()? {
                mounted = changes.change(&router, router_routes)?;
            }
        }
    }
    if mounted {
        notes.push("Mounted GET /account/security (with setup, confirm and disable) and /login/mfa in the router.".to_owned());
    } else {
        notes.push(routes_note(accounts));
    }
    let pages = root.join("src/pages/auth.rs");
    let linked = accounts == Accounts::Starter
        && pages.try_exists()?
        && changes.change(&pages, |source| Ok(dashboard_link(source)))?;
    if !linked {
        notes.push("Link /account/security from your account pages.".to_owned());
    }
    let main = root.join("src/main.rs");
    if main.try_exists()? && changes.change(&main, main_hook)? {
        notes.push("`<application> admin:mfa-reset <email>` (for example `cargo run -- admin:mfa-reset member@example.com`) removes an account's second factor and signs it out everywhere.".to_owned());
    } else {
        notes.push("To reset a lost phone from the server, call `controllers::mfa::admin::run().await` at the start of an async `main` and exit with the status it returns: `<application> admin:mfa-reset <email>`.".to_owned());
    }
    Ok(notes)
}

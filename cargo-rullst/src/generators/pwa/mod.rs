//! `cargo rullst make:pwa`: makes an HTML application an installable
//! Progressive Web App, wired into the starter's own router and layouts.
//!
//! The whole plan is validated before anything is written. A file that
//! exists with the content this run would write is left alone and an already
//! wired router or layout is not changed again, so running the same command
//! twice changes nothing; a generated file that differs (edited, or another
//! `--name`) stops the run before any write.
use super::consumer_files::{self as writes, Edit};
use super::source_walk;
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::{
    error::Error,
    io,
    path::{Path, PathBuf},
};

mod files;
mod head;
mod report;
mod texts;
mod values;
mod wiring;

use values::{PwaFlags, PwaValues, invalid};

pub(crate) fn command() -> Command {
    let value = |name: &'static str, help: &'static str| Arg::new(name).long(name).help(help);
    Command::new("make:pwa")
        .about("Makes an HTML application an installable Progressive Web App (manifest, icons, service worker, offline page)")
        .arg(value("name", "Application name; defaults to the Cargo package name in words"))
        .arg(value("short-name", "Name under the home-screen icon; defaults to the name when it has at most 12 characters"))
        .arg(value("theme-color", "Browser and title-bar colour as #RRGGBB [default: #0f172a]"))
        .arg(value("background-color", "Splash-screen background as #RRGGBB [default: #020617]"))
        .arg(value("lang", "Language of the application, such as en or pt-BR [default: en]"))
        .arg(value("start-url", "Path the installed app opens [default: /]"))
        .arg(
            Arg::new("android")
                .long("android")
                .action(ArgAction::SetTrue)
                .requires_all(["host", "package-id"])
                .help("Also write android/twa-manifest.json for a Google Play package (Bubblewrap)"),
        )
        .arg(value("host", "With --android: the HTTPS host of the site, such as app.example.com").requires("android"))
        .arg(value("package-id", "With --android: the application-owned Android package name, such as com.acme.notes").requires("android"))
        .arg(
            Arg::new("router")
                .long("router")
                .value_name("PATH")
                .value_parser(clap::value_parser!(PathBuf))
                .help("Source file that builds the router, such as src/routes.rs: mounts the PWA routes there when src/ has no single `.merge_axum(rullst::health::health_router())`"),
        )
}

fn flags(matches: &ArgMatches) -> PwaFlags {
    let text = |name: &str| matches.get_one::<String>(name).cloned();
    PwaFlags {
        name: text("name"),
        short_name: text("short-name"),
        theme_color: text("theme-color"),
        background_color: text("background-color"),
        lang: text("lang"),
        start_url: text("start-url"),
        android: matches.get_flag("android"),
        host: text("host"),
        package_id: text("package-id"),
        router: matches.get_one::<PathBuf>("router").cloned(),
    }
}

#[cfg_attr(mutants, mutants::skip)]
pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let plan = plan(&std::env::current_dir()?, &flags(matches))?;
    writes::apply(&plan.edits)?;
    print!("{}", plan.report());
    Ok(())
}

/// The validated edits and what they do.
#[derive(Default)]
struct Plan {
    edits: Vec<Edit>,
    created: Vec<String>,
    changed: Vec<String>,
    /// Layouts that register the worker since this run.
    layouts: usize,
    /// Of those, layouts that keep their own manifest link.
    own_manifest: usize,
    /// Layouts that already registered the worker.
    present: usize,
    /// Files whose documents are e-mail messages, left alone.
    skipped: Vec<String>,
    /// Why the PWA routes are not mounted, when they are not.
    unmounted: Option<&'static str>,
    /// `--lang` when the generated texts are not in that language.
    untranslated: Option<String>,
}

/// `path` relative to `root`, with `/` separators, for messages.
fn relative(root: &Path, path: &Path) -> String {
    super::slash_path(path.strip_prefix(root).unwrap_or(path))
}

fn package_name(manifest: &str) -> io::Result<String> {
    let manifest = toml::from_str::<toml::Value>(manifest)
        .map_err(|_| invalid("Cargo.toml is not valid TOML"))?;
    if manifest
        .get("dependencies")
        .and_then(|dependencies| dependencies.get("rullst"))
        .is_none()
    {
        return Err(invalid(
            "make:pwa must run in a Rullst application: Cargo.toml has no `rullst` dependency",
        ));
    }
    manifest
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| invalid("Cargo.toml [package].name must be a string"))
}

/// Adds the creation of `relative` to `plan` unless the file already holds
/// exactly `content`; a different file stops the run.
fn create(plan: &mut Plan, root: &Path, relative: &str, content: Vec<u8>) -> io::Result<()> {
    let path = root.join(relative);
    if path.try_exists()? {
        if std::fs::read(&path)? == content {
            return Ok(());
        }
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "{relative} already exists with other content; make:pwa never overwrites it. Keep your version (the rest of the PWA is wired by hand, see PWA.md) or move it away and run make:pwa again"
            ),
        ));
    }
    plan.edits.push(Edit::create_bytes(path, content)?);
    plan.created.push(relative.to_owned());
    Ok(())
}

fn change(plan: &mut Plan, root: &Path, path: PathBuf, old: String, new: String) {
    plan.changed.push(relative(root, &path));
    plan.edits.push(Edit::replace(path, old, new));
}

/// Test code, which holds no page layout.
fn is_test_source(name: &str) -> bool {
    let stem = name.trim_end_matches(".rs");
    let file = stem.rsplit('/').next().unwrap_or(stem);
    file == "tests"
        || file.ends_with("_tests")
        || file.ends_with("_test")
        || stem.split('/').any(|part| part == "tests")
}

/// A module of e-mail messages, by its directory or file name
/// (`src/mail/site_mail.rs`, `src/emails/welcome.rs`).
fn is_mail_path(name: &str) -> bool {
    const NAMES: [&str; 8] = [
        "mail",
        "mails",
        "email",
        "emails",
        "mailer",
        "mailers",
        "mailable",
        "mailables",
    ];
    name.trim_end_matches(".rs").split('/').any(|part| {
        NAMES.contains(&part)
            || NAMES.iter().any(|mail| {
                part.ends_with(&format!("_{mail}")) || part.starts_with(&format!("{mail}_"))
            })
    })
}

/// A file that sends e-mail, whose string documents are messages.
fn sends_mail(source: &str) -> bool {
    ["rullst::mail", "rullst_mail", "lettre::"]
        .iter()
        .any(|marker| source.contains(marker))
}

/// A source of this crate: its path, its text and the text with the PWA.
type Source = (PathBuf, String, String);

/// Every source of this crate, with the head tags in its page layouts;
/// generated, test and e-mail sources are skipped.
fn wire_layouts(
    root: &Path,
    paths: Vec<PathBuf>,
    is_library: bool,
    values: &PwaValues,
    plan: &mut Plan,
) -> Result<Vec<Source>, Box<dyn Error>> {
    let mut files = Vec::new();
    for path in paths {
        let name = relative(root, &path);
        let generated = name == "src/pwa.rs" || name.starts_with("src/pwa/");
        let other_crate = name.starts_with("src/bin/") || (is_library && name == "src/main.rs");
        if generated || other_crate || is_test_source(&name) {
            continue;
        }
        let source = writes::read(&path)?;
        if is_mail_path(&name) {
            if source.to_ascii_lowercase().contains("</head>") {
                plan.skipped.push(name);
            }
            files.push((path, source.clone(), source));
            continue;
        }
        let wiring = head::wire(&source, &values.theme_color, !sends_mail(&source))
            .map_err(|error| invalid(format!("make:pwa could not read {name}: {error}")))?;
        plan.layouts += wiring.wired;
        plan.own_manifest += wiring.own_manifest;
        plan.present += wiring.present;
        if wiring.skipped > 0 {
            plan.skipped.push(name);
        }
        files.push((path, source, wiring.source));
    }
    Ok(files)
}

/// The index in `files` of the `--router` file: a source of this crate
/// under `src/`, as `wire_layouts` lists them.
fn router_file(root: &Path, files: &[Source], router: &Path) -> io::Result<usize> {
    let shown = super::slash_path(router);
    let wanted = std::fs::canonicalize(root.join(router))
        .map_err(|_| invalid(format!("--router {shown}: no such file")))?;
    files
        .iter()
        .position(|(path, _, _)| *path == wanted)
        .ok_or_else(|| {
            invalid(format!(
                "--router {shown} is not a source of this crate: name a .rs file under src/ that builds the router (not src/bin/, src/main.rs beside a src/lib.rs, a test or a make:pwa file)"
            ))
        })
}

/// Mounts the PWA routes after the one health merge of the crate, in any
/// module, or in the `router` file; without exactly one merge and without
/// `router`, the report says what to mount.
fn mount_routes(
    files: &mut [Source],
    plan: &mut Plan,
    router: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    let mut merges = Vec::new();
    for (index, (_, _, updated)) in files.iter().enumerate() {
        let (count, mounted) = wiring::health_merges(updated).unwrap_or_default();
        if mounted {
            return Ok(());
        }
        merges.extend(std::iter::repeat_n(index, count));
    }
    if let Some((path, _, updated)) = router.and_then(|index| files.get_mut(index)) {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        *updated = wiring::mount_router_in(updated)
            .map_err(|error| invalid(format!("--router {name}: {error}")))?;
        return Ok(());
    }
    match merges.as_slice() {
        [index] => {
            if let Some((_, _, updated)) = files.get_mut(*index)
                && let Some(with_routes) = wiring::mount_router(updated)?
            {
                *updated = with_routes;
            }
        }
        [] => plan.unmounted = Some("src/ has no `.merge_axum(rullst::health::health_router())`"),
        _ => plan.unmounted = Some("src/ merges `rullst::health::health_router()` more than once"),
    }
    Ok(())
}

fn plan(root: &Path, flags: &PwaFlags) -> Result<Plan, Box<dyn Error>> {
    // Links above the project are the developer's; links beneath it are refused.
    let root = &writes::project_root(root)?;
    let package = package_name(&writes::read(&root.join("Cargo.toml"))?)?;
    let values = PwaValues::resolve(flags, &package)?;
    let library = root.join("src/lib.rs");
    let crate_root = if library.try_exists()? {
        library
    } else {
        root.join("src/main.rs")
    };
    let is_library = crate_root.ends_with("lib.rs");
    let (texts, translated) = texts::for_lang(&values.lang);
    let mut plan = Plan {
        untranslated: (!translated).then(|| values.lang.clone()),
        ..Plan::default()
    };

    let sources = source_walk::rust_sources(&root.join("src"));
    if let Some(reason) = sources.incomplete {
        return Err(invalid(format!(
            "make:pwa could not read every source file: {reason}"
        ))
        .into());
    }
    let mut files = wire_layouts(root, sources.files, is_library, &values, &mut plan)?;
    if !files.iter().any(|(path, _, _)| *path == crate_root) {
        return Err(invalid("make:pwa needs src/main.rs or src/lib.rs").into());
    }
    let router = flags
        .router
        .as_deref()
        .map(|router| router_file(root, &files, router))
        .transpose()?;
    mount_routes(&mut files, &mut plan, router)?;
    for (path, original, mut updated) in files {
        if path == crate_root
            && let Some(declared) = wiring::declare_module(&updated, is_library)?
        {
            updated = declared;
        }
        if updated != original {
            change(&mut plan, root, path, original, updated);
        }
    }

    // The smoke test of `cargo rullst new` (a test source, so not a layout
    // above) checks the offline page too.
    let smoke = root.join(crate::generators::smoke_pages::FILE);
    if smoke.try_exists()? {
        let listed = writes::read(&smoke)?;
        if let Some(updated) =
            crate::generators::smoke_pages::with_pages(&listed, &[("/offline", 200)])
        {
            change(&mut plan, root, smoke, listed, updated);
        }
    }

    create(
        &mut plan,
        root,
        "src/pwa.rs",
        files::module(&values, crate::generators::frontend::uses_tailwind(root)).into_bytes(),
    )?;
    create(
        &mut plan,
        root,
        "src/pwa/sw.js",
        files::service_worker(root).into_bytes(),
    )?;
    create(
        &mut plan,
        root,
        "static/pwa.js",
        files::registration(&texts).into_bytes(),
    )?;
    create(
        &mut plan,
        root,
        "static/pwa.css",
        files::STYLESHEET.as_bytes().to_vec(),
    )?;
    create(
        &mut plan,
        root,
        "static/manifest.webmanifest",
        files::web_manifest(&values)?.into_bytes(),
    )?;
    for (path, bytes) in files::ICONS {
        create(&mut plan, root, path, bytes.to_vec())?;
    }
    if let Some(android) = &values.android {
        create(
            &mut plan,
            root,
            "android/twa-manifest.json",
            files::twa_manifest(&values, android)?.into_bytes(),
        )?;
        create(
            &mut plan,
            root,
            "PWA-ANDROID.md",
            files::android_readme(android).into_bytes(),
        )?;
    }
    create(&mut plan, root, "PWA.md", files::readme().into_bytes())?;
    Ok(plan)
}

#[cfg(test)]
mod router_tests;
#[cfg(test)]
mod style_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod wiring_tests;

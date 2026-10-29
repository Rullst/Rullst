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
struct Plan {
    edits: Vec<Edit>,
    created: Vec<String>,
    changed: Vec<String>,
    /// Layouts this run wires.
    layouts: usize,
    /// Layouts that already linked the manifest.
    present: usize,
}

impl Plan {
    fn report(&self) -> String {
        if self.edits.is_empty() {
            return "The PWA files and wiring are already in place; nothing changed.\n".to_owned();
        }
        let mut report = String::from("Progressive Web App added.\n");
        for path in &self.created {
            report.push_str(&format!("  created  {path}\n"));
        }
        for path in &self.changed {
            report.push_str(&format!("  updated  {path}\n"));
        }
        if self.layouts == 0 {
            report.push_str(&format!(
                "{} page layout(s) already link the manifest and register /sw.js.\n",
                self.present
            ));
        } else {
            report.push_str(&format!(
                "{} page layout(s) link the manifest and register /sw.js.\n",
                self.layouts
            ));
        }
        report.push_str(
            "Replace the placeholder icons in static/icons/ and read PWA.md before release.\n",
        );
        report
    }
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
    let mut plan = Plan {
        edits: Vec::new(),
        created: Vec::new(),
        changed: Vec::new(),
        layouts: 0,
        present: 0,
    };

    let sources = source_walk::rust_sources(&root.join("src"));
    if let Some(reason) = sources.incomplete {
        return Err(invalid(format!(
            "make:pwa could not read every source file: {reason}"
        ))
        .into());
    }
    let mut crate_root_source = None;
    let mut present = 0;
    for path in sources.files {
        let name = relative(root, &path);
        let generated = name == "src/pwa.rs" || name.starts_with("src/pwa/");
        let other_crate = name.starts_with("src/bin/") || (is_library && name == "src/main.rs");
        if generated || other_crate {
            continue;
        }
        let source = writes::read(&path)?;
        let wiring = head::wire(&source, &values.theme_color)
            .map_err(|error| invalid(format!("make:pwa could not read {name}: {error}")))?;
        plan.layouts += wiring.wired;
        present += wiring.present;
        plan.present += wiring.present;
        if path == crate_root {
            crate_root_source = Some((source, wiring.source));
        } else if wiring.wired > 0 {
            change(&mut plan, root, path, source, wiring.source);
        }
    }
    let Some((original, mut updated)) = crate_root_source else {
        return Err(invalid("make:pwa needs src/main.rs or src/lib.rs").into());
    };
    if plan.layouts + present == 0 {
        return Err(invalid(
            "make:pwa found no HTML page layout (an html! document, a <head> in a string or render_page_with_lang) in src/; the JSON API starter serves no pages to install",
        )
        .into());
    }
    if let Some(mounted) = wiring::mount_router(&updated)? {
        updated = mounted;
    }
    if let Some(declared) = wiring::declare_module(&updated, is_library)? {
        updated = declared;
    }
    if updated != original {
        change(&mut plan, root, crate_root, original, updated);
    }

    create(
        &mut plan,
        root,
        "src/pwa.rs",
        files::module(&values).into_bytes(),
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
        files::REGISTRATION.as_bytes().to_vec(),
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
mod tests;

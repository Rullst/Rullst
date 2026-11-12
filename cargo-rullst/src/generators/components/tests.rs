//! `cargo rullst ui add` on temporary projects: what it copies, registers
//! and declares, that a second run changes nothing, that a changed copy is
//! kept without `--force`, and what the report says about Tailwind.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::catalog::{self, COMPONENTS, NAMES};
use super::plan::{MODULE_HEADER, MODULE_PATH};
use super::{ComponentsError, Failure, Request, execute, refused};
use crate::ui::style::Style;
use std::path::Path;

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let mut all = vec![
        (
            "Cargo.toml",
            "[package]\nname = \"shop\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13.0.0-alpha.5\"\n",
        ),
        ("src/main.rs", "mod routes;\n\nfn main() {}\n"),
    ];
    all.extend_from_slice(files);
    for (path, text) in all {
        let path = root.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    root
}

fn request(names: &[&str]) -> Request {
    Request {
        names: names.iter().map(|name| (*name).to_owned()).collect(),
        force: false,
        dry_run: false,
    }
}

fn add(root: &Path, request: &Request) -> String {
    execute(root, request, Style::PLAIN).unwrap()
}

fn read(root: &Path, path: &str) -> String {
    std::fs::read_to_string(root.join(path)).unwrap()
}

#[test]
fn the_catalog_matches_the_kit_and_its_scripts() {
    assert_eq!(COMPONENTS.len(), NAMES.len());
    let kit = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generators/components/kit");
    let mut sources: Vec<String> = std::fs::read_dir(&kit)
        .unwrap()
        .filter_map(|entry| {
            let name = entry.unwrap().file_name().into_string().unwrap();
            name.strip_suffix(".rs").map(str::to_owned)
        })
        .collect();
    sources.sort();
    let mut names: Vec<String> = NAMES.iter().map(|name| (*name).to_owned()).collect();
    names.sort();
    assert_eq!(sources, names);
    for (component, name) in COMPONENTS.iter().zip(NAMES) {
        assert_eq!(component.name, name);
        assert_eq!(
            std::fs::read_to_string(kit.join(format!("{name}.rs"))).unwrap(),
            component.source
        );
        let loads = format!("rullst::assets::url(\"ui/{name}.js\")");
        assert_eq!(
            component.source.contains(&loads),
            component.script.is_some(),
            "{name}"
        );
        for requirement in component.requires {
            let required = catalog::find(requirement).unwrap();
            assert!(
                required.requires.is_empty(),
                "{name} needs a flat requirement"
            );
            assert!(component.source.contains(&format!("super::{requirement}")));
        }
        assert!(
            component
                .source
                .contains(&format!("cargo rullst ui add {name}"))
        );
    }
}

#[test]
fn resolve_adds_requirements_in_catalog_order_and_reports_unknown_names() {
    let (components, added) = catalog::resolve(["menu", "dialog"]).unwrap();
    let names: Vec<&str> = components.iter().map(|component| component.name).collect();
    assert_eq!(names, ["button", "dialog", "menu"]);
    assert_eq!(added, ["button"]);
    let (_, added) = catalog::resolve(["dialog", "button"]).unwrap();
    assert!(added.is_empty());
    assert_eq!(
        catalog::resolve(["card", "carousel"]).unwrap_err(),
        "carousel"
    );
}

#[test]
fn add_copies_registers_and_declares_the_components() {
    let root = project(&[]);
    let text = add(root.path(), &request(&["dialog"]));
    let root = root.path();
    assert_eq!(
        read(root, "src/ui/dialog.rs"),
        catalog::find("dialog").unwrap().source
    );
    assert_eq!(
        read(root, "src/ui/button.rs"),
        catalog::find("button").unwrap().source
    );
    assert_eq!(
        read(root, "static/ui/dialog.js"),
        catalog::find("dialog").unwrap().script.unwrap()
    );
    assert!(!root.join("static/ui/button.js").exists());
    assert_eq!(
        read(root, MODULE_PATH),
        format!("{MODULE_HEADER}\npub mod button;\npub mod dialog;\n")
    );
    let main = read(root, "src/main.rs");
    assert!(
        main.contains("// UI components (cargo rullst ui add).\nmod ui;\n"),
        "{main}"
    );
    syn::parse_file(&main).unwrap();
    for line in [
        "src/ui/button.rs",
        "src/ui/dialog.rs",
        "static/ui/dialog.js",
        "src/ui/mod.rs",
        "declare `mod ui;`",
        "also copied, because the others use it: button",
        "does not use Tailwind CSS",
    ] {
        assert!(text.contains(line), "{line}\n{text}");
    }
    assert!(!text.contains("Dry run"));

    // Again: nothing changes.
    let again = add(root, &request(&["dialog", "button"]));
    assert!(again.contains("Nothing to change"), "{again}");
    assert!(
        again.contains(
            "already up to date: src/ui/button.rs, src/ui/dialog.rs, static/ui/dialog.js"
        )
    );
    assert_eq!(read(root, "src/main.rs"), main);

    // A third component joins the existing registration.
    add(root, &request(&["badge"]));
    assert!(
        read(root, MODULE_PATH).ends_with("pub mod badge;\npub mod button;\npub mod dialog;\n")
    );
    assert_eq!(read(root, "src/main.rs").matches("mod ui;").count(), 1);
}

#[test]
fn a_library_declares_a_public_ui_module_and_keeps_its_own_module_file() {
    let root = project(&[
        ("src/lib.rs", "pub mod pages;\n"),
        ("src/ui/mod.rs", "//! Mine.\n\npub mod layout;\n"),
    ]);
    add(root.path(), &request(&["tabs"]));
    let lib = read(root.path(), "src/lib.rs");
    assert!(lib.contains("pub mod ui;"), "{lib}");
    assert!(!read(root.path(), "src/main.rs").contains("mod ui"));
    assert_eq!(
        read(root.path(), MODULE_PATH),
        "//! Mine.\n\npub mod layout;\npub mod tabs;\n"
    );
}

#[test]
fn a_changed_copy_is_kept_unless_forced() {
    let root = project(&[("src/ui/badge.rs", "// my badge\n")]);
    let root = root.path();
    let Err(Failure::Components(ComponentsError::Modified(files))) =
        execute(root, &request(&["badge", "card"]), Style::PLAIN)
    else {
        panic!("a changed copy must stop the command");
    };
    assert_eq!(files, ["src/ui/badge.rs"]);
    assert!(refused(root, &request(&["badge"])));
    assert!(!refused(root, &request(&["card"])));
    // Nothing was written, not even the other component.
    assert!(!root.join("src/ui/card.rs").exists());
    assert!(!root.join(MODULE_PATH).exists());

    let mut preview = request(&["badge"]);
    preview.dry_run = true;
    let text = add(root, &preview);
    assert!(
        text.contains("src/ui/badge.rs differ from this CLI's copies"),
        "{text}"
    );
    assert!(text.contains("- // my badge"), "{text}");
    assert_eq!(read(root, "src/ui/badge.rs"), "// my badge\n");

    let mut forced = request(&["badge"]);
    forced.force = true;
    assert!(!refused(root, &forced));
    let text = add(root, &forced);
    assert!(
        text.contains("replace the changed copy (--force)"),
        "{text}"
    );
    assert!(!text.contains("differ from this CLI's copies"));
    assert_eq!(
        read(root, "src/ui/badge.rs"),
        catalog::find("badge").unwrap().source
    );
}

#[test]
fn a_dry_run_writes_nothing_and_shows_the_files() {
    let root = project(&[]);
    let mut preview = request(&["toast"]);
    preview.dry_run = true;
    let text = add(root.path(), &preview);
    assert!(text.contains("would copy the toast component"));
    assert!(text.contains("+++ static/ui/toast.js (new file)"));
    assert!(text.contains("Dry run: nothing was written."));
    assert!(text.contains("render ui::toast::region once per page"));
    assert!(!root.path().join("src/ui").exists());
    assert!(!root.path().join("static").exists());
    assert_eq!(
        read(root.path(), "src/main.rs"),
        "mod routes;\n\nfn main() {}\n"
    );
}

#[test]
fn the_report_follows_the_projects_tailwind_and_rullst_version() {
    let managed = project(&[(
        "assets/css/app.css",
        "@import \"tailwindcss\" source(none);\n@source \"../../src/**/*.rs\";\n",
    )]);
    let text = add(managed.path(), &request(&["button"]));
    assert!(!text.contains("Tailwind"), "{text}");

    let own = project(&[("tailwind.config.js", "module.exports = {};\n")]);
    let text = add(own.path(), &request(&["button"]));
    assert!(
        text.contains("make sure its sources include src/ui/*.rs"),
        "{text}"
    );
    assert!(!text.contains("does not use Tailwind"));

    let old = project(&[(
        "Cargo.lock",
        "version = 4\n\n[[package]]\nname = \"rullst\"\nversion = \"13.0.0-alpha.4\"\n",
    )]);
    let text = add(old.path(), &request(&["badge"]));
    assert!(
        !text.contains("cargo rullst upgrade"),
        "no module, no warning"
    );
    let text = add(old.path(), &request(&["menu"]));
    assert!(text.contains("run `cargo rullst upgrade` first"), "{text}");
}

#[test]
fn outside_a_project_or_without_a_crate_root_it_says_so() {
    let empty = tempfile::tempdir().unwrap();
    assert!(matches!(
        execute(empty.path(), &request(&["badge"]), Style::PLAIN),
        Err(Failure::Project)
    ));
    let root = project(&[]);
    std::fs::remove_file(root.path().join("src/main.rs")).unwrap();
    let text = add(root.path(), &request(&["badge"]));
    assert!(
        text.contains("the crate root (no src/main.rs or src/lib.rs):\n    mod ui;"),
        "{text}"
    );
    let broken = project(&[("src/main.rs", "fn main( {\n")]);
    let text = add(broken.path(), &request(&["badge"]));
    assert!(
        text.contains("src/main.rs (it does not parse as Rust):\n    mod ui;"),
        "{text}"
    );
    assert!(matches!(
        execute(root.path(), &request(&["carousel"]), Style::PLAIN),
        Err(Failure::Components(ComponentsError::Unknown(name))) if name == "carousel"
    ));
    assert!(refused(root.path(), &request(&["carousel"])));
}

#[test]
fn the_command_parses_names_all_and_flags() {
    let parse = |arguments: &[&str]| {
        let matches = crate::command().try_get_matches_from(arguments)?;
        let (_, ui) = matches.subcommand().unwrap();
        let (name, add) = ui.subcommand().unwrap();
        assert_eq!(name, "add");
        Ok::<_, clap::Error>(Request::from_matches(add))
    };
    let all = parse(&["rullst", "ui", "add", "--all", "--dry-run"]).unwrap();
    assert_eq!(all.names, NAMES);
    assert!(all.dry_run && !all.force);
    let some = parse(&["rullst", "ui", "add", "tabs", "card", "--force"]).unwrap();
    assert_eq!(some.names, ["tabs", "card"]);
    assert!(some.force && !some.dry_run);
    assert!(parse(&["rullst", "ui", "add"]).is_err());
    assert!(parse(&["rullst", "ui", "add", "carousel"]).is_err());
    assert!(parse(&["rullst", "ui", "add", "card", "--all"]).is_err());
    assert!(
        crate::command()
            .try_get_matches_from(["rullst", "ui", "list"])
            .is_ok()
    );
    let list = super::report::list(Style::PLAIN);
    for component in &COMPONENTS {
        assert!(list.contains(component.name) && list.contains(component.summary));
    }
    assert!(list.contains("src/ui/menu.rs + static/ui/menu.js, uses button"));
}

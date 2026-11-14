//! The offline page follows the project's choice: plain tags and its one
//! `pwa-offline` class without Tailwind, utility classes with it.

use super::{PwaFlags, PwaValues, files, plan, writes};
use crate::blueprints::{self, BLANK_BLUEPRINT_ID};
use std::fs;
use std::path::{Path, PathBuf};

fn values() -> PwaValues {
    PwaValues {
        name: "Field Notes".to_owned(),
        short_name: "Notes".to_owned(),
        theme_color: "#112233".to_owned(),
        background_color: "#445566".to_owned(),
        lang: "en".to_owned(),
        start_url: "/".to_owned(),
        android: None,
    }
}

#[test]
fn a_tailwind_project_gets_the_same_page_with_utility_classes() {
    let plain = files::module(&values(), false);
    let styled = files::module(&values(), true);
    assert!(plain.contains(r#"<main class="pwa-offline">"#));
    assert!(styled.contains(r#"<main class="pwa-offline mx-auto flex"#));
    assert!(styled.contains(r#"<h1 class="text-2xl font-semibold text-slate-100">"#));
    assert!(styled.contains(r#"<p class="text-slate-300">"#));
    assert!(styled.contains(r#"<a href="" class="font-medium text-sky-400"#));
    assert!(!styled.contains("__"));
    // Only the four tags differ.
    let unstyled = styled
        .replace(
            " mx-auto flex min-h-full max-w-xl flex-col justify-center gap-4 px-6 py-16 text-center",
            "",
        )
        .replace(r#" class="text-2xl font-semibold text-slate-100""#, "")
        .replace(r#" class="text-slate-300""#, "")
        .replace(
            r#" class="font-medium text-sky-400 underline underline-offset-4 hover:text-sky-300""#,
            "",
        );
    assert_eq!(unstyled, plain);
    syn::parse_file(&styled).unwrap();
}

fn blank_starter(temporary: &Path) -> PathBuf {
    let root = writes::symlinked_project(temporary);
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"notes\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13\"\n",
    )
    .unwrap();
    blueprints::apply(
        BLANK_BLUEPRINT_ID,
        &root,
        "notes",
        "notes",
        false,
        false,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .unwrap();
    root
}

#[test]
fn make_pwa_reads_the_choice_from_the_project() {
    let temporary = tempfile::tempdir().unwrap();
    let plain = blank_starter(temporary.path());
    writes::apply(&plan(&plain, &PwaFlags::default()).unwrap().edits).unwrap();
    let module = fs::read_to_string(plain.join("src/pwa.rs")).unwrap();
    assert!(module.contains(r#"<main class="pwa-offline">"#), "{module}");

    let other = tempfile::tempdir().unwrap();
    let styled = blank_starter(other.path());
    fs::create_dir_all(styled.join("assets/css")).unwrap();
    fs::write(
        styled.join("assets/css/app.css"),
        "@import \"tailwindcss\" source(none);\n",
    )
    .unwrap();
    writes::apply(&plan(&styled, &PwaFlags::default()).unwrap().edits).unwrap();
    let module = fs::read_to_string(styled.join("src/pwa.rs")).unwrap();
    assert!(
        module.contains("<main class=\"pwa-offline mx-auto flex"),
        "{module}"
    );
}

#[test]
fn the_worker_stores_the_htmx_client_the_project_ships() {
    // A project created before 13.0.0-alpha.5 vendors htmx 1.9.12, a newer
    // one htmx 2.0.11; the worker lists only the file that exists.
    for (client, other) in [
        ("htmx-1.9.12.min.js", "htmx-2.0.11.min.js"),
        ("htmx-2.0.11.min.js", "htmx-1.9.12.min.js"),
    ] {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("static")).unwrap();
        fs::write(root.path().join("static").join(client), "/* htmx */").unwrap();
        let worker = files::service_worker(root.path());
        assert!(
            worker.contains(&format!("  '/static/{client}',\n")),
            "{worker}"
        );
        assert!(!worker.contains(other), "{worker}");
    }
}

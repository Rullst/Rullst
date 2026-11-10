//! `make:pwa --router <file>`: the routes are mounted in the named file when
//! the automatic search finds no health merge or several.
use super::*;
use crate::blueprints::{self, SAAS_BLUEPRINT_ID};
use std::fs;

const MANIFEST: &str = "[package]\nname = \"acme-notes\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13\"\n";
const HEALTH: &str = ".merge_axum(rullst::health::health_router())";

fn saas(temporary: &Path) -> PathBuf {
    let root = writes::symlinked_project(temporary);
    fs::write(root.join("Cargo.toml"), MANIFEST).unwrap();
    blueprints::apply(
        SAAS_BLUEPRINT_ID,
        &root,
        "acme-notes",
        "acme_notes",
        false,
        false,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .unwrap();
    root
}

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap()
}

fn with_router(router: &str) -> PwaFlags {
    PwaFlags {
        router: Some(PathBuf::from(router)),
        ..PwaFlags::default()
    }
}

#[test]
fn the_router_flag_is_read_from_the_command_line() {
    let matches = command()
        .try_get_matches_from(["make:pwa", "--router", "src/routes.rs"])
        .unwrap();
    assert_eq!(flags(&matches).router, Some(PathBuf::from("src/routes.rs")));
    let matches = command().try_get_matches_from(["make:pwa"]).unwrap();
    assert_eq!(flags(&matches).router, None);
}

#[test]
fn a_router_without_health_probes_is_mounted_inside_its_header_layer() {
    let temporary = tempfile::tempdir().unwrap();
    let root = saas(temporary.path());
    let probes = format!(
        "    // `/health` and `/ready` for container, Kubernetes and PaaS probes.\n    {HEALTH}\n"
    );
    let main = read(&root, "src/main.rs");
    assert!(main.contains(&probes));
    fs::write(root.join("src/main.rs"), main.replace(&probes, "")).unwrap();

    let planned = plan(&root, &with_router("src/main.rs")).unwrap();
    assert_eq!(planned.unmounted, None);
    assert!(!planned.report().contains("To do: The PWA routes"));
    writes::apply(&planned.edits).unwrap();
    let mounted = read(&root, "src/main.rs");
    assert!(
        mounted.contains(
            "\n    // Service worker and offline page (cargo rullst make:pwa).\n    .merge_axum(crate::pwa::router().into_axum())\n    .layer(rullst::server::from_fn(rullst::security::headers_middleware))"
        ),
        "{mounted}"
    );
    syn::parse_file(&mounted).unwrap();

    // Mounted once: a second run, with or without the flag, changes nothing.
    for flags in [with_router("src/main.rs"), PwaFlags::default()] {
        let again = plan(&root, &flags).unwrap();
        assert!(again.edits.is_empty());
        assert_eq!(again.unmounted, None);
    }
}

#[test]
fn the_named_router_wins_over_merges_in_other_files() {
    let temporary = tempfile::tempdir().unwrap();
    let root = saas(temporary.path());
    // A second module that merges the probes: the search is ambiguous.
    let routes = "pub fn api() -> rullst::Router {\n    rullst::Router::new()\n        .merge_axum(rullst::health::health_router())\n}\n";
    fs::write(root.join("src/routes.rs"), routes).unwrap();
    let main = read(&root, "src/main.rs").replace("pub mod pages;", "pub mod pages;\nmod routes;");
    fs::write(root.join("src/main.rs"), &main).unwrap();
    let searched = plan(&root, &PwaFlags::default()).unwrap();
    assert_eq!(
        searched.unmounted,
        Some("src/ merges `rullst::health::health_router()` more than once")
    );
    assert!(searched.report().contains("--router <file>"));

    let named = plan(&root, &with_router("src/routes.rs")).unwrap();
    assert_eq!(named.unmounted, None);
    assert!(named.changed.contains(&"src/routes.rs".to_owned()));
    writes::apply(&named.edits).unwrap();
    assert_eq!(
        read(&root, "src/routes.rs"),
        "pub fn api() -> rullst::Router {\n    rullst::Router::new()\n        .merge_axum(rullst::health::health_router())\n        // Service worker and offline page (cargo rullst make:pwa).\n        .merge_axum(crate::pwa::router().into_axum())\n}\n"
    );
    assert!(!read(&root, "src/main.rs").contains("crate::pwa::router()"));
    // An absolute spelling of the same file is accepted.
    let absolute = root.join("src/routes.rs");
    let again = plan(&root, &with_router(absolute.to_str().unwrap())).unwrap();
    assert!(again.edits.is_empty());
}

#[test]
fn a_router_that_is_not_a_crate_source_or_has_no_anchor_is_refused() {
    let temporary = tempfile::tempdir().unwrap();
    let root = saas(temporary.path());
    let refused = |router: &str| {
        plan(&root, &with_router(router))
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default()
    };
    assert_eq!(
        refused("src/missing.rs"),
        "--router src/missing.rs: no such file"
    );
    fs::create_dir_all(root.join("src/bin")).unwrap();
    fs::write(root.join("src/bin/tool.rs"), "fn main() {}\n").unwrap();
    assert!(
        refused("src/bin/tool.rs")
            .starts_with("--router src/bin/tool.rs is not a source of this crate"),
        "{}",
        refused("src/bin/tool.rs")
    );
    assert!(refused("Cargo.toml").contains("is not a source of this crate"));

    // A file with neither anchor: nothing is written.
    fs::write(root.join("src/plain.rs"), "pub fn nothing() {}\n").unwrap();
    let error = refused("src/plain.rs");
    assert!(
        error.starts_with("--router plain.rs: it has neither a `.merge_axum(rullst::health::health_router())` nor exactly one `.layer(...)` of `rullst::security::headers_middleware`"),
        "{error}"
    );
    assert!(!root.join("src/pwa.rs").exists());

    // Two merges in the named file are as ambiguous as two files.
    let twice = format!("fn a(r: R) -> R {{ r{HEALTH} }}\nfn b(r: R) -> R {{ r{HEALTH} }}\n");
    fs::write(root.join("src/plain.rs"), twice).unwrap();
    assert!(
        refused("src/plain.rs").contains("it merges `rullst::health::health_router()` 2 times")
    );
}

#[test]
fn the_header_anchor_needs_exactly_one_header_layer() {
    let once = "fn r(x: R) -> R {\n    x.route(\"/\", h)\n        .layer(from_fn(rullst::security::headers_middleware))\n}\n";
    let mounted = wiring::mount_router_in(once).unwrap();
    assert_eq!(
        mounted,
        "fn r(x: R) -> R {\n    x.route(\"/\", h)\n        // Service worker and offline page (cargo rullst make:pwa).\n        .merge_axum(crate::pwa::router().into_axum())\n        .layer(from_fn(rullst::security::headers_middleware))\n}\n"
    );
    // Already mounted: unchanged.
    assert_eq!(wiring::mount_router_in(&mounted).unwrap(), mounted);
    let twice = once.replace(
        ".layer(from_fn(rullst::security::headers_middleware))",
        ".layer(from_fn(headers_middleware)).layer(from_fn(rullst::security::headers_middleware))",
    );
    assert!(wiring::mount_router_in(&twice).is_err());
    // Another layer, or the name outside a layer argument, is no anchor.
    assert!(
        wiring::mount_router_in(&once.replace("headers_middleware", "csrf_middleware")).is_err()
    );
    assert!(wiring::mount_router_in("fn r(x: R) -> R { x.merge(headers_middleware) }").is_err());
    assert!(wiring::mount_router_in("fn r(x: R) -> R { x.layer(a, headers_middleware) }").is_err());
}

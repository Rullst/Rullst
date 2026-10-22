//! The files `make:pwa` writes, rendered from validated values.
use super::values::PwaValues;
use serde::Serialize;
use std::path::Path;

pub(super) const SERVICE_WORKER: &str = include_str!("assets/sw.js");
pub(super) const REGISTRATION: &str = include_str!("assets/pwa.js");
const MODULE: &str = include_str!("assets/pwa.rs.template");
const README: &str = include_str!("assets/PWA.md.template");

/// The line of `sw.js` that lists the files stored at install time.
pub(super) const PRECACHE_LINE: &str = "const PRECACHE = ['/offline'];";

/// The placeholder icons: the Rullst logo on the slate-950 canvas of the
/// retired Omni icon step, pre-rendered so that no image library is needed.
pub(super) const ICONS: [(&str, &[u8]); 4] = [
    (
        "static/icons/icon-192.png",
        include_bytes!("icons/icon-192.png"),
    ),
    (
        "static/icons/icon-512.png",
        include_bytes!("icons/icon-512.png"),
    ),
    (
        "static/icons/maskable-512.png",
        include_bytes!("icons/maskable-512.png"),
    ),
    (
        "static/icons/apple-touch-icon.png",
        include_bytes!("icons/apple-touch-icon.png"),
    ),
];

/// Static files the starters ship that the offline page may need; each is
/// stored by the worker only when the project has it.
const OPTIONAL_PRECACHE: [&str; 3] = [
    "static/rullst.css",
    "static/htmx-1.9.12.min.js",
    "static/rullst.png",
];

#[derive(Serialize)]
struct Icon {
    src: &'static str,
    sizes: &'static str,
    #[serde(rename = "type")]
    kind: &'static str,
    purpose: &'static str,
}

#[derive(Serialize)]
struct WebManifest<'a> {
    id: &'a str,
    name: &'a str,
    short_name: &'a str,
    start_url: &'a str,
    scope: &'a str,
    display: &'a str,
    theme_color: &'a str,
    background_color: &'a str,
    lang: &'a str,
    icons: [Icon; 3],
}

/// `static/manifest.webmanifest`. `id` stays `/` when `start_url` changes,
/// so installed apps and stores keep recognizing the application.
pub(super) fn web_manifest(values: &PwaValues) -> Result<String, serde_json::Error> {
    let icon = |src, sizes, purpose| Icon {
        src,
        sizes,
        kind: "image/png",
        purpose,
    };
    let manifest = WebManifest {
        id: "/",
        name: &values.name,
        short_name: &values.short_name,
        start_url: &values.start_url,
        scope: "/",
        display: "standalone",
        theme_color: &values.theme_color,
        background_color: &values.background_color,
        lang: &values.lang,
        icons: [
            icon("/static/icons/icon-192.png", "192x192", "any"),
            icon("/static/icons/icon-512.png", "512x512", "any"),
            icon("/static/icons/maskable-512.png", "512x512", "maskable"),
        ],
    };
    Ok(serde_json::to_string_pretty(&manifest)? + "\n")
}

/// `src/pwa/sw.js` with the files to store at install: the offline page,
/// the PWA's own files and the starter files that exist in `root`.
pub(super) fn service_worker(root: &Path) -> String {
    let mut urls = vec![
        "/offline".to_owned(),
        "/static/manifest.webmanifest".to_owned(),
        "/static/pwa.js".to_owned(),
        "/static/icons/icon-192.png".to_owned(),
        "/static/icons/icon-512.png".to_owned(),
    ];
    urls.extend(
        OPTIONAL_PRECACHE
            .iter()
            .filter(|path| root.join(path).is_file())
            .map(|path| format!("/{path}")),
    );
    let list = urls
        .iter()
        .map(|url| format!("  '{url}',\n"))
        .collect::<String>();
    SERVICE_WORKER.replacen(PRECACHE_LINE, &format!("const PRECACHE = [\n{list}];"), 1)
}

/// `src/pwa.rs`.
pub(super) fn module(values: &PwaValues) -> String {
    MODULE
        .replace("__THEME_COLOR__", &values.theme_color)
        .replace("__LANG__", &values.lang)
}

/// `PWA.md`.
pub(super) fn readme() -> String {
    README.replace("__ANDROID__", "")
}

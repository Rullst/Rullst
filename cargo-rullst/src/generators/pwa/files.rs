//! The files `make:pwa` writes, rendered from validated values.
use super::texts::{self, Texts};
use super::values::{AndroidValues, PwaValues};
use serde::Serialize;
use std::path::Path;

pub(super) const SERVICE_WORKER: &str = include_str!("assets/sw.js");
pub(super) const REGISTRATION: &str = include_str!("assets/pwa.js");
/// The default look of the update notice, loaded by `pwa.js`.
pub(super) const STYLESHEET: &str = include_str!("assets/pwa.css");
const MODULE: &str = include_str!("assets/pwa.rs.template");
const README: &str = include_str!("assets/PWA.md.template");
const ANDROID_README: &str = include_str!("assets/PWA.android.md.template");

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

/// The comment the worker's list puts after a placeholder file.
fn placeholder_note(url: &str) -> &'static str {
    match url {
        "/static/icons/icon-192.png" | "/static/icons/icon-512.png" => {
            " // placeholder icon (the Rullst logo): replace it"
        }
        "/static/rullst.png" => " // the starter's Rullst logo, the icon of Core's page",
        _ => "",
    }
}

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
        .map(|url| format!("  '{url}',{}\n", placeholder_note(url)))
        .collect::<String>();
    SERVICE_WORKER.replacen(PRECACHE_LINE, &format!("const PRECACHE = [\n{list}];"), 1)
}

/// The offline page's tags with Tailwind utility classes, for a project that
/// uses Tailwind (`uses_tailwind`); otherwise the page keeps the plain tags
/// and its one `pwa-offline` class. The colours suit Core's dark page.
const TAILWIND_OFFLINE: [(&str, &str); 4] = [
    (
        r#"<main class="pwa-offline">"#,
        r#"<main class="pwa-offline mx-auto flex min-h-full max-w-xl flex-col justify-center gap-4 px-6 py-16 text-center">"#,
    ),
    (
        r#"<h1>"__OFFLINE_HEADING__"</h1>"#,
        r#"<h1 class="text-2xl font-semibold text-slate-100">"__OFFLINE_HEADING__"</h1>"#,
    ),
    (
        r#"<p>"__OFFLINE_MESSAGE__"</p>"#,
        r#"<p class="text-slate-300">"__OFFLINE_MESSAGE__"</p>"#,
    ),
    (
        r#"<p><a href="">"#,
        r#"<p><a href="" class="font-medium text-sky-400 underline underline-offset-4 hover:text-sky-300">"#,
    ),
];

/// `src/pwa.rs`, with the offline page in the language of `--lang`, styled
/// with Tailwind classes when the project uses Tailwind.
pub(super) fn module(values: &PwaValues, tailwind: bool) -> String {
    let (texts, _) = texts::for_lang(&values.lang);
    let mut module = MODULE.to_owned();
    if tailwind {
        for (plain, styled) in TAILWIND_OFFLINE {
            module = module.replacen(plain, styled, 1);
        }
    }
    module
        .replace("__THEME_COLOR__", &values.theme_color)
        .replace("__LANG__", &values.lang)
        .replace("__OFFLINE_TITLE__", texts.offline_title)
        .replace("__OFFLINE_HEADING__", texts.offline_heading)
        .replace("__OFFLINE_MESSAGE__", texts.offline_message)
        .replace("__OFFLINE_RETRY__", texts.offline_retry)
}

/// `static/pwa.js`, with the update notice in `texts`.
pub(super) fn registration(texts: &Texts) -> String {
    let english = texts::ENGLISH;
    let line = |key: &str, text: &str| format!("    {key}: '{}',\n", texts::javascript(text));
    REGISTRATION
        .replacen(
            &line("update", english.update_message),
            &line("update", texts.update_message),
            1,
        )
        .replacen(
            &line("reload", english.update_reload),
            &line("reload", texts.update_reload),
            1,
        )
}

/// `PWA.md`.
pub(super) fn readme() -> String {
    README.to_owned()
}

/// `PWA-ANDROID.md`, the Google Play steps for `--android`.
pub(super) fn android_readme(android: &AndroidValues) -> String {
    ANDROID_README
        .replace("__HOST__", &android.host)
        .replace("__PACKAGE_ID__", &android.package_id)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SigningKey {
    path: &'static str,
    alias: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TwaManifest<'a> {
    package_id: &'a str,
    host: &'a str,
    name: &'a str,
    launcher_name: &'a str,
    display: &'a str,
    theme_color: &'a str,
    theme_color_dark: &'a str,
    navigation_color: &'a str,
    navigation_color_dark: &'a str,
    navigation_divider_color: &'a str,
    navigation_divider_color_dark: &'a str,
    background_color: &'a str,
    enable_notifications: bool,
    start_url: &'a str,
    icon_url: String,
    maskable_icon_url: String,
    web_manifest_url: String,
    full_scope_url: String,
    splash_screen_fade_out_duration: u32,
    signing_key: SigningKey,
    app_version: &'a str,
    app_version_code: u32,
    fallback_type: &'a str,
    generator_app: &'a str,
}

/// `android/twa-manifest.json` for Bubblewrap, from the web manifest values.
/// The signing key entries are placeholders: the application's own upload
/// key, kept outside the repository.
pub(super) fn twa_manifest(
    values: &PwaValues,
    android: &AndroidValues,
) -> Result<String, serde_json::Error> {
    let origin = format!("https://{}", android.host);
    let manifest = TwaManifest {
        package_id: &android.package_id,
        host: &android.host,
        name: &values.name,
        launcher_name: &values.short_name,
        display: "standalone",
        theme_color: &values.theme_color,
        theme_color_dark: &values.theme_color,
        navigation_color: &values.theme_color,
        navigation_color_dark: &values.theme_color,
        navigation_divider_color: &values.theme_color,
        navigation_divider_color_dark: &values.theme_color,
        background_color: &values.background_color,
        enable_notifications: false,
        start_url: &values.start_url,
        icon_url: format!("{origin}/static/icons/icon-512.png"),
        maskable_icon_url: format!("{origin}/static/icons/maskable-512.png"),
        web_manifest_url: format!("{origin}/static/manifest.webmanifest"),
        full_scope_url: format!("{origin}/"),
        splash_screen_fade_out_duration: 300,
        signing_key: SigningKey {
            path: "/private/path/upload.keystore",
            alias: "upload",
        },
        app_version: "1",
        app_version_code: 1,
        fallback_type: "customtabs",
        generator_app: "cargo-rullst make:pwa",
    };
    Ok(serde_json::to_string_pretty(&manifest)? + "\n")
}

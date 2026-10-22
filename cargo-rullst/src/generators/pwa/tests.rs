use super::*;
use crate::blueprints::{
    self, BLANK_BLUEPRINT_ID, BLOG_BLUEPRINT_ID, ERP_BLUEPRINT_ID, LMS_BLUEPRINT_ID,
    PORTFOLIO_BLUEPRINT_ID, SAAS_BLUEPRINT_ID,
};
use std::fs;

const MANIFEST: &str = "[package]\nname = \"acme-field-notes\"\nversion = \"0.3.1\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13\"\n";

fn starter(temporary: &Path, blueprint: usize, api: bool, hot: bool) -> PathBuf {
    let root = writes::symlinked_project(temporary);
    fs::write(root.join("Cargo.toml"), MANIFEST).unwrap();
    blueprints::apply(
        blueprint,
        &root,
        "acme-field-notes",
        "acme_field_notes",
        api,
        hot,
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

#[test]
fn every_html_starter_is_wired_once_and_a_second_run_changes_nothing() {
    for (blueprint, hot, layouts, entry) in [
        (BLANK_BLUEPRINT_ID, false, 1, "src/main.rs"),
        (BLANK_BLUEPRINT_ID, true, 1, "src/lib.rs"),
        (BLOG_BLUEPRINT_ID, false, 2, "src/main.rs"),
        (LMS_BLUEPRINT_ID, false, 5, "src/main.rs"),
        (SAAS_BLUEPRINT_ID, false, 3, "src/main.rs"),
        (ERP_BLUEPRINT_ID, false, 1, "src/main.rs"),
        (PORTFOLIO_BLUEPRINT_ID, false, 1, "src/main.rs"),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let root = starter(temporary.path(), blueprint, false, hot);
        let planned = plan(&root, &PwaFlags::default()).unwrap();
        assert_eq!(planned.layouts, layouts, "blueprint {blueprint}");
        assert!(
            planned.changed.contains(&entry.to_owned()),
            "{:?}",
            planned.changed
        );
        writes::apply(&planned.edits).unwrap();

        let entry_source = read(&root, entry);
        assert_eq!(entry_source.matches("crate::pwa::router()").count(), 1);
        let declaration = if hot {
            "\npub mod pwa;\n"
        } else {
            "\nmod pwa;\n"
        };
        assert!(entry_source.contains(declaration), "{entry_source}");
        for created in [
            "src/pwa.rs",
            "src/pwa/sw.js",
            "static/pwa.js",
            "static/manifest.webmanifest",
            "static/icons/maskable-512.png",
            "PWA.md",
        ] {
            assert!(root.join(created).is_file(), "{created}");
        }
        syn::parse_file(&read(&root, "src/pwa.rs")).unwrap();
        for path in planned.changed {
            syn::parse_file(&read(&root, &path)).unwrap();
        }

        let again = plan(&root, &PwaFlags::default()).unwrap();
        assert!(again.edits.is_empty(), "blueprint {blueprint}");
        assert_eq!(again.layouts, 0);
        assert_eq!(
            again.report(),
            "The PWA files and wiring are already in place; nothing changed.\n"
        );
    }
}

#[test]
fn the_blank_starter_gets_the_core_layout_and_its_own_files() {
    let temporary = tempfile::tempdir().unwrap();
    let root = starter(temporary.path(), BLANK_BLUEPRINT_ID, false, false);
    let planned = plan(&root, &PwaFlags::default()).unwrap();
    assert_eq!(
        planned.created,
        [
            "src/pwa.rs",
            "src/pwa/sw.js",
            "static/pwa.js",
            "static/manifest.webmanifest",
            "static/icons/icon-192.png",
            "static/icons/icon-512.png",
            "static/icons/maskable-512.png",
            "static/icons/apple-touch-icon.png",
            "PWA.md",
        ]
    );
    assert_eq!(planned.changed, ["src/main.rs"]);
    let report = planned.report();
    assert!(report.starts_with("Progressive Web App added.\n  created  src/pwa.rs\n"));
    assert!(report.contains("  updated  src/main.rs\n1 page layout(s) link the manifest"));
    writes::apply(&planned.edits).unwrap();

    let main = read(&root, "src/main.rs");
    assert!(main.contains("use rullst::htmx::{HtmxRequest, render_page_with_head};"));
    assert!(main.contains(
        "    render_page_with_head(\n        &htmx,\n        \"en\",\n        \"Welcome to Rullst\",\n        crate::pwa::head(),\n        content,\n    )\n}"
    ));
    assert!(main.contains(
        ".merge_axum(rullst::health::health_router())\n    // Service worker and offline page (cargo rullst make:pwa).\n    .merge_axum(crate::pwa::router().into_axum())\n    .layer(rullst::server::from_fn(rullst::security::headers_middleware))"
    ));
    // The blank starter ships its stylesheet, htmx and logo, so the worker stores them.
    let worker = read(&root, "src/pwa/sw.js");
    assert!(worker.contains("const PRECACHE = [\n  '/offline',\n  '/static/manifest.webmanifest',\n  '/static/pwa.js',\n  '/static/icons/icon-192.png',\n  '/static/icons/icon-512.png',\n  '/static/rullst.css',\n  '/static/htmx-1.9.12.min.js',\n  '/static/rullst.png',\n];"));
    assert_eq!(
        fs::read(root.join("static/icons/icon-512.png")).unwrap(),
        include_bytes!("icons/icon-512.png")
    );
    let manifest =
        serde_json::from_str::<serde_json::Value>(&read(&root, "static/manifest.webmanifest"))
            .unwrap();
    assert_eq!(manifest["name"], "Acme Field Notes");
    assert_eq!(manifest["short_name"], "Acme");
}

#[test]
fn an_edited_generated_file_stops_the_run_before_any_write() {
    let temporary = tempfile::tempdir().unwrap();
    let root = starter(temporary.path(), BLOG_BLUEPRINT_ID, false, false);
    writes::apply(&plan(&root, &PwaFlags::default()).unwrap().edits).unwrap();
    fs::write(root.join("static/pwa.js"), "// mine\n").unwrap();
    let page = read(&root, "src/pages/blog.rs");
    let error = plan(&root, &PwaFlags::default()).err().unwrap().to_string();
    assert!(
        error.starts_with(
            "static/pwa.js already exists with other content; make:pwa never overwrites it."
        ),
        "{error}"
    );
    // Another name renders another manifest, which is refused the same way.
    fs::write(root.join("static/pwa.js"), files::REGISTRATION).unwrap();
    let renamed = PwaFlags {
        name: Some("Other".to_owned()),
        ..PwaFlags::default()
    };
    let error = plan(&root, &renamed).err().unwrap().to_string();
    assert!(
        error.starts_with("static/manifest.webmanifest already exists"),
        "{error}"
    );
    assert_eq!(read(&root, "src/pages/blog.rs"), page);
}

#[test]
fn projects_without_pages_or_a_known_router_are_refused() {
    let temporary = tempfile::tempdir().unwrap();
    let api = starter(temporary.path(), BLANK_BLUEPRINT_ID, true, false);
    let error = plan(&api, &PwaFlags::default()).err().unwrap().to_string();
    assert!(
        error.contains("the JSON API starter serves no pages to install"),
        "{error}"
    );
    assert!(!api.join("src/pwa.rs").exists());

    let temporary = tempfile::tempdir().unwrap();
    let custom = starter(temporary.path(), ERP_BLUEPRINT_ID, false, false);
    let main =
        read(&custom, "src/main.rs").replace(".merge_axum(rullst::health::health_router())", "");
    fs::write(custom.join("src/main.rs"), &main).unwrap();
    let error = plan(&custom, &PwaFlags::default())
        .err()
        .unwrap()
        .to_string();
    assert!(
        error.starts_with("make:pwa did not find exactly one"),
        "{error}"
    );

    fs::write(custom.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    let error = plan(&custom, &PwaFlags::default())
        .err()
        .unwrap()
        .to_string();
    assert_eq!(
        error,
        "make:pwa must run in a Rullst application: Cargo.toml has no `rullst` dependency"
    );
    fs::write(
        custom.join("Cargo.toml"),
        "[dependencies]\nrullst = \"13\"\n",
    )
    .unwrap();
    let error = plan(&custom, &PwaFlags::default())
        .err()
        .unwrap()
        .to_string();
    assert_eq!(error, "Cargo.toml [package].name must be a string");
    fs::write(custom.join("Cargo.toml"), "[package\n").unwrap();
    let error = plan(&custom, &PwaFlags::default())
        .err()
        .unwrap()
        .to_string();
    assert_eq!(error, "Cargo.toml is not valid TOML");
}

#[test]
fn flags_are_read_from_the_command_line() {
    let matches = command()
        .try_get_matches_from([
            "make:pwa",
            "--name",
            "Field Notes",
            "--short-name",
            "Notes",
            "--theme-color",
            "#112233",
            "--background-color",
            "#445566",
            "--lang",
            "pt-BR",
            "--start-url",
            "/?source=pwa",
        ])
        .unwrap();
    let flags = flags(&matches);
    assert_eq!(flags.name.as_deref(), Some("Field Notes"));
    assert_eq!(flags.short_name.as_deref(), Some("Notes"));
    assert_eq!(flags.theme_color.as_deref(), Some("#112233"));
    assert_eq!(flags.background_color.as_deref(), Some("#445566"));
    assert_eq!(flags.lang.as_deref(), Some("pt-BR"));
    assert_eq!(flags.start_url.as_deref(), Some("/?source=pwa"));
}

#[test]
fn values_have_documented_defaults_and_bounds() {
    let defaults = PwaValues::resolve(&PwaFlags::default(), "acme_chat").unwrap();
    assert_eq!(
        defaults,
        PwaValues {
            name: "Acme Chat".to_owned(),
            short_name: "Acme Chat".to_owned(),
            theme_color: "#0f172a".to_owned(),
            background_color: "#020617".to_owned(),
            lang: "en".to_owned(),
            start_url: "/".to_owned(),
            android: None,
        }
    );
    let long = PwaValues::resolve(&PwaFlags::default(), "internationalization-console").unwrap();
    assert_eq!(long.short_name, "Internationa");
    // Twelve characters still fit under the icon.
    let twelve = PwaValues::resolve(&PwaFlags::default(), "acme-fieldsx").unwrap();
    assert_eq!(twelve.short_name, "Acme Fieldsx");
    for (name, lang) in [("x".repeat(64), "fil"), ("Notes".to_owned(), "en-abcdefgh")] {
        let flags = PwaFlags {
            name: Some(name.clone()),
            lang: Some(lang.to_owned()),
            ..PwaFlags::default()
        };
        let values = PwaValues::resolve(&flags, "x").unwrap();
        assert_eq!((values.name, values.lang), (name, lang.to_owned()));
    }
    let given = PwaFlags {
        theme_color: Some("#ABCDEF".to_owned()),
        lang: Some("zh-Hant".to_owned()),
        short_name: Some("Exactly12chr".to_owned()),
        ..PwaFlags::default()
    };
    let given = PwaValues::resolve(&given, "x").unwrap();
    assert_eq!(given.theme_color, "#abcdef");
    assert_eq!(given.short_name, "Exactly12chr");

    let refused = |flags: PwaFlags| {
        PwaValues::resolve(&flags, "acme")
            .err()
            .unwrap()
            .to_string()
    };
    for name in ["", " Lead", ".hidden", "a/b", "<script>", &"x".repeat(65)] {
        let message = refused(PwaFlags {
            name: Some(name.to_owned()),
            ..PwaFlags::default()
        });
        assert!(
            message.starts_with("--name must contain 1-64 letters"),
            "{name}: {message}"
        );
    }
    assert!(
        refused(PwaFlags {
            short_name: Some(String::new()),
            ..PwaFlags::default()
        })
        .starts_with("--short-name")
    );
    for color in ["0f172a", "#0f172", "#0f172aa", "#0g172a", "red"] {
        let message = refused(PwaFlags {
            background_color: Some(color.to_owned()),
            ..PwaFlags::default()
        });
        assert_eq!(
            message,
            "--background-color must be a #RRGGBB colour such as #0f172a"
        );
    }
    assert!(
        refused(PwaFlags {
            theme_color: Some("#12".to_owned()),
            ..PwaFlags::default()
        })
        .starts_with("--theme-color")
    );
    for lang in [
        "e",
        "engl",
        "en_US",
        "en-",
        "en-x",
        "pt-BR-toolongvalue",
        "1n",
        "en-U$",
    ] {
        let message = refused(PwaFlags {
            lang: Some(lang.to_owned()),
            ..PwaFlags::default()
        });
        assert_eq!(
            message, "--lang must be a language tag such as en or pt-BR",
            "{lang}"
        );
    }
    for start in [
        "",
        "home",
        "//evil.example",
        "/a b",
        "/\"x",
        "/'x",
        "/a\\b",
        "/<x>",
        "/\u{7}",
        &format!("/{}", "a".repeat(512)),
    ] {
        let message = refused(PwaFlags {
            start_url: Some(start.to_owned()),
            ..PwaFlags::default()
        });
        assert!(
            message.starts_with("--start-url must be a path"),
            "{start:?}"
        );
    }
    assert!(
        PwaValues::resolve(
            &PwaFlags {
                start_url: Some(format!("/{}", "a".repeat(511))),
                ..PwaFlags::default()
            },
            "x"
        )
        .is_ok()
    );
}

#[test]
fn the_router_and_module_wiring_recognize_their_own_output() {
    let router = "fn router() -> Router {\n    if missing {\n        return Router::new().merge_axum(rullst::health::health_router());\n    }\n    routes![]\n    .merge_axum(rullst :: health :: health_router())\n    .layer(x)\n}\n";
    let mounted = wiring::mount_router(router).unwrap().unwrap();
    assert_eq!(
        mounted,
        "fn router() -> Router {\n    if missing {\n        return Router::new().merge_axum(rullst::health::health_router());\n    }\n    routes![]\n    .merge_axum(rullst :: health :: health_router())\n    // Service worker and offline page (cargo rullst make:pwa).\n    .merge_axum(crate::pwa::router().into_axum())\n    .layer(x)\n}\n"
    );
    assert_eq!(wiring::mount_router(&mounted).unwrap(), None);
    let indented = "fn r() -> R {\n    let router = routes![]\n        .merge_axum(rullst::health::health_router());\n    router\n}";
    assert_eq!(
        wiring::mount_router(indented).unwrap().unwrap(),
        "fn r() -> R {\n    let router = routes![]\n        .merge_axum(rullst::health::health_router())\n        // Service worker and offline page (cargo rullst make:pwa).\n        .merge_axum(crate::pwa::router().into_axum());\n    router\n}"
    );
    let two = "fn a() -> R { r.merge_axum(rullst::health::health_router()) }\nfn b() -> R { r.merge_axum(rullst::health::health_router()) }";
    assert!(wiring::mount_router(two).is_err());
    // Only a merge counts; a call elsewhere is not the router.
    assert!(wiring::mount_router("fn a() { let h = rullst::health::health_router(); }").is_err());
    assert!(wiring::mount_router("fn a() { r.merge(rullst::health::health_router()); }").is_err());

    let declared = wiring::declare_module("fn main() {}", false)
        .unwrap()
        .unwrap();
    assert_eq!(
        declared,
        "fn main() {}\n\n// Web app manifest, service worker and offline page (cargo rullst make:pwa).\nmod pwa;\n"
    );
    assert_eq!(wiring::declare_module(&declared, false).unwrap(), None);
    let library = wiring::declare_module("pub mod pages;\n", true)
        .unwrap()
        .unwrap();
    assert!(library.ends_with("\npub mod pwa;\n"));
    assert_eq!(wiring::declare_module("mod pwa { }", true).unwrap(), None);
}

#[test]
fn rendered_files_hold_the_validated_values() {
    let values = PwaValues {
        name: "Field Notes".to_owned(),
        short_name: "Notes".to_owned(),
        theme_color: "#112233".to_owned(),
        background_color: "#445566".to_owned(),
        lang: "pt-BR".to_owned(),
        start_url: "/?source=pwa".to_owned(),
        android: None,
    };
    assert_eq!(
        files::web_manifest(&values).unwrap(),
        r##"{
  "id": "/",
  "name": "Field Notes",
  "short_name": "Notes",
  "start_url": "/?source=pwa",
  "scope": "/",
  "display": "standalone",
  "theme_color": "#112233",
  "background_color": "#445566",
  "lang": "pt-BR",
  "icons": [
    {
      "src": "/static/icons/icon-192.png",
      "sizes": "192x192",
      "type": "image/png",
      "purpose": "any"
    },
    {
      "src": "/static/icons/icon-512.png",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "any"
    },
    {
      "src": "/static/icons/maskable-512.png",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "maskable"
    }
  ]
}
"##
    );
    let module = files::module(&values);
    assert!(module.contains(r##"<meta name="theme-color" content="#112233" />"##));
    assert!(module.contains(r#"render_page_with_lang(&htmx, "pt-BR", "Offline", content)"#));
    assert!(!module.contains("__"));
    syn::parse_file(&module).unwrap();
    // The worker keeps the lines src/pwa.rs and make:pwa replace.
    assert_eq!(
        files::SERVICE_WORKER
            .matches("const VERSION = 'dev';")
            .count(),
        1
    );
    assert!(module.contains("\"const VERSION = 'dev';\""));
    assert_eq!(
        files::SERVICE_WORKER.matches(files::PRECACHE_LINE).count(),
        1
    );
    let temporary = tempfile::tempdir().unwrap();
    let worker = files::service_worker(temporary.path());
    assert!(worker.contains("const PRECACHE = [\n  '/offline',\n  '/static/manifest.webmanifest',\n  '/static/pwa.js',\n  '/static/icons/icon-192.png',\n  '/static/icons/icon-512.png',\n];"));
    assert!(!worker.contains(files::PRECACHE_LINE));
    let readme = files::readme();
    assert!(readme.starts_with("# Progressive Web App\n"));
    assert!(!readme.contains("__"));
    // The registration script and worker keep their security branches.
    for branch in [
        "if (request.method !== 'GET') return;",
        "if (url.origin !== self.location.origin) return;",
        "(no-store|private)",
        "event.data.type === 'SKIP_WAITING'",
    ] {
        assert!(files::SERVICE_WORKER.contains(branch), "{branch}");
    }
    for branch in [
        "container.register('/sw.js', { scope: '/' })",
        "worker.postMessage({ type: 'SKIP_WAITING' })",
        "if (!reloadRequested) return;",
    ] {
        assert!(files::REGISTRATION.contains(branch), "{branch}");
    }
    assert!(!files::REGISTRATION.contains("setAttribute('style'"));
}

const PLAY_FLAGS: [&str; 5] = [
    "--android",
    "--host",
    "notes.acme.dev",
    "--package-id",
    "dev.acme.notes",
];

fn android_flags() -> PwaFlags {
    flags(
        &command()
            .try_get_matches_from(std::iter::once("make:pwa").chain(PLAY_FLAGS))
            .unwrap(),
    )
}

#[test]
fn android_files_can_be_added_after_the_web_files() {
    let temporary = tempfile::tempdir().unwrap();
    let root = starter(temporary.path(), PORTFOLIO_BLUEPRINT_ID, false, false);
    writes::apply(&plan(&root, &PwaFlags::default()).unwrap().edits).unwrap();
    let android = plan(&root, &android_flags()).unwrap();
    assert_eq!(
        android.created,
        ["android/twa-manifest.json", "PWA-ANDROID.md"]
    );
    assert!(android.changed.is_empty());
    writes::apply(&android.edits).unwrap();
    let notes = read(&root, "PWA-ANDROID.md");
    assert!(notes.starts_with("# Google Play (Trusted Web Activity)\n"));
    assert!(notes.contains("RULLST_ANDROID_ASSET_LINKS=\"dev.acme.notes=<app signing SHA-256>"));
    assert!(notes.contains("`https://notes.acme.dev/.well-known/assetlinks.json`"));
    assert!(!notes.contains("__"));
    // Both runs are complete now: neither changes anything again.
    assert!(plan(&root, &android_flags()).unwrap().edits.is_empty());
    assert!(plan(&root, &PwaFlags::default()).unwrap().edits.is_empty());
    let module = read(&root, "src/pwa.rs");
    assert!(module.contains("match rullst::pwa::AssetLinks::from_project() {"));
}

#[test]
fn the_bubblewrap_configuration_mirrors_the_web_manifest() {
    let values = PwaValues::resolve(
        &PwaFlags {
            name: Some("Field Notes".to_owned()),
            short_name: Some("Notes".to_owned()),
            start_url: Some("/?source=twa".to_owned()),
            ..android_flags()
        },
        "x",
    )
    .unwrap();
    let android = values.android.clone().unwrap();
    assert_eq!(
        files::twa_manifest(&values, &android).unwrap(),
        r##"{
  "packageId": "dev.acme.notes",
  "host": "notes.acme.dev",
  "name": "Field Notes",
  "launcherName": "Notes",
  "display": "standalone",
  "themeColor": "#0f172a",
  "themeColorDark": "#0f172a",
  "navigationColor": "#0f172a",
  "navigationColorDark": "#0f172a",
  "navigationDividerColor": "#0f172a",
  "navigationDividerColorDark": "#0f172a",
  "backgroundColor": "#020617",
  "enableNotifications": false,
  "startUrl": "/?source=twa",
  "iconUrl": "https://notes.acme.dev/static/icons/icon-512.png",
  "maskableIconUrl": "https://notes.acme.dev/static/icons/maskable-512.png",
  "webManifestUrl": "https://notes.acme.dev/static/manifest.webmanifest",
  "fullScopeUrl": "https://notes.acme.dev/",
  "splashScreenFadeOutDuration": 300,
  "signingKey": {
    "path": "/private/path/upload.keystore",
    "alias": "upload"
  },
  "appVersion": "1",
  "appVersionCode": 1,
  "fallbackType": "customtabs",
  "generatorApp": "cargo-rullst make:pwa"
}
"##
    );
}

#[test]
fn android_values_are_validated() {
    assert!(
        command()
            .try_get_matches_from(["make:pwa", "--android"])
            .is_err()
    );
    assert!(
        command()
            .try_get_matches_from(["make:pwa", "--host", "notes.acme.dev"])
            .is_err()
    );
    let resolved = PwaValues::resolve(&android_flags(), "x").unwrap();
    assert_eq!(
        resolved.android,
        Some(values::AndroidValues {
            host: "notes.acme.dev".to_owned(),
            package_id: "dev.acme.notes".to_owned(),
        })
    );
    assert_eq!(
        PwaValues::resolve(&PwaFlags::default(), "x")
            .unwrap()
            .android,
        None
    );
    let refused = |host: Option<&str>, package: Option<&str>| {
        PwaValues::resolve(
            &PwaFlags {
                android: true,
                host: host.map(str::to_owned),
                package_id: package.map(str::to_owned),
                ..PwaFlags::default()
            },
            "x",
        )
        .err()
        .unwrap()
        .to_string()
    };
    assert_eq!(
        refused(None, Some("dev.acme.notes")),
        "--android needs --host, the HTTPS host of the site"
    );
    assert_eq!(
        refused(Some("notes.acme.dev"), None),
        "--android needs --package-id, an application-owned Android package name"
    );
    for host in [
        "localhost",
        "https://notes.acme.dev",
        "notes.acme.dev/app",
        "notes.acme.dev:8443",
        "Notes.acme.dev",
        "-notes.acme.dev",
        "notes-.acme.dev",
        "notes..dev",
        "10.0.0.1",
        "notes.acme.d3v",
        &format!("{}.dev", "a".repeat(64)),
        &format!(
            "{}.dev",
            ["a", "b", "c", "d", "e"]
                .map(|letter| letter.repeat(60))
                .join(".")
        ),
    ] {
        assert!(
            refused(Some(host), Some("dev.acme.notes")).starts_with("--host must be"),
            "{host}"
        );
    }
    for host in [
        "a.dev",
        &format!("{}.dev", "a".repeat(63)),
        "app-1.acme.dev",
    ] {
        assert!(
            PwaValues::resolve(
                &PwaFlags {
                    host: Some(host.to_owned()),
                    ..android_flags()
                },
                "x"
            )
            .is_ok(),
            "{host}"
        );
    }
    for package in [
        "notes",
        "Dev.acme",
        "dev.1acme",
        "dev.acme-notes",
        "dev..notes",
        &format!("dev.{}", "a".repeat(147)),
    ] {
        assert!(
            refused(Some("notes.acme.dev"), Some(package)).starts_with("--package-id must be"),
            "{package}"
        );
    }
    assert_eq!(
        refused(Some("notes.acme.dev"), Some("com.example.notes")),
        "--package-id may not use the com.example placeholder namespace; use an application-owned name"
    );
    for package in [
        "dev.acme_2.notes",
        "a.b",
        &format!("dev.{}", "a".repeat(146)),
    ] {
        assert!(
            PwaValues::resolve(
                &PwaFlags {
                    package_id: Some(package.to_owned()),
                    ..android_flags()
                },
                "x"
            )
            .is_ok(),
            "{package}"
        );
    }
}

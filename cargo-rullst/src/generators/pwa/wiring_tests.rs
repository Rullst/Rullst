//! `make:pwa` in applications laid out unlike the starters: the router in its
//! own module, a layout with its own manifest and e-mail templates.
use super::*;
use std::fs;

const MANIFEST: &str = "[package]\nname = \"capoeira-site\"\nversion = \"1.0.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13\"\n";

const MAIN: &str =
    "mod mail;\nmod pages;\nmod routes;\n\n#[cfg(test)]\nmod security_tests;\n\nfn main() {}\n";

const ROUTES: &str = "pub fn router() -> rullst::Router {\n    rullst::Router::new()\n        .merge_axum(rullst::health::health_router())\n}\n";

const LAYOUT: &str = r#"use rullst::html::Markup;

pub fn page(title: &str, main: Markup) -> Markup {
    rullst::html! {
        <!DOCTYPE html>
        <html lang="pt-BR">
            <head>
                <title>{title}</title>
                <link rel="manifest" href="/static/site.webmanifest" />
            </head>
            <body>{main}</body>
        </html>
    }
}
"#;

const MAIL: &str = r##"pub fn html(subject: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>{subject}</title></head><body style="margin:0"></body></html>"#
    )
}
"##;

/// An application whose router and layout live outside the crate root.
fn application(temporary: &Path) -> PathBuf {
    let root = writes::symlinked_project(temporary);
    for (path, content) in [
        ("Cargo.toml", MANIFEST),
        ("src/main.rs", MAIN),
        ("src/routes.rs", ROUTES),
        ("src/pages/mod.rs", "pub mod layout;\n"),
        ("src/pages/layout.rs", LAYOUT),
        ("src/mail/mod.rs", "pub mod site_mail;\n"),
        ("src/mail/site_mail.rs", MAIL),
        (
            "src/security_tests.rs",
            "#[test]\nfn head() { assert!(\"<head></head>\".contains(\"head\")); }\n",
        ),
    ] {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    root
}

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap()
}

#[test]
fn a_router_module_a_layout_with_its_manifest_and_mail_are_handled() {
    let temporary = tempfile::tempdir().unwrap();
    let root = application(temporary.path());
    let planned = plan(&root, &PwaFlags::default()).unwrap();
    assert_eq!(
        (planned.layouts, planned.own_manifest, planned.present),
        (1, 1, 0)
    );
    assert_eq!(planned.skipped, ["src/mail/site_mail.rs"]);
    assert_eq!(planned.unmounted, None);
    assert_eq!(
        planned.changed,
        ["src/main.rs", "src/pages/layout.rs", "src/routes.rs"]
    );
    let report = planned.report();
    for expected in [
        "  skipped  src/mail/site_mail.rs: an e-mail message, not a page layout\n",
        "1 page layout(s) now register /sw.js.\n1 of them keep their own manifest link",
    ] {
        assert!(report.contains(expected), "{report}");
    }
    assert!(!report.contains("To do:"), "{report}");
    writes::apply(&planned.edits).unwrap();

    // The layout that already linked its manifest now registers the worker.
    assert!(read(&root, "src/pages/layout.rs").contains(
        "<link rel=\"manifest\" href=\"/static/site.webmanifest\" />\n            <script src=\"/static/pwa.js\" defer></script></head>"
    ));
    // E-mail and test code are never edited.
    assert_eq!(read(&root, "src/mail/site_mail.rs"), MAIL);
    assert!(!read(&root, "src/security_tests.rs").contains("pwa"));
    // The routes follow the health merge in src/routes.rs.
    assert_eq!(
        read(&root, "src/routes.rs"),
        "pub fn router() -> rullst::Router {\n    rullst::Router::new()\n        .merge_axum(rullst::health::health_router())\n        // Service worker and offline page (cargo rullst make:pwa).\n        .merge_axum(crate::pwa::router().into_axum())\n}\n"
    );
    // The module joins the declarations, before the test module and `main`.
    assert_eq!(
        read(&root, "src/main.rs"),
        "mod mail;\nmod pages;\nmod routes;\n\n// Web app manifest, service worker and offline page (cargo rullst make:pwa).\nmod pwa;\n\n#[cfg(test)]\nmod security_tests;\n\nfn main() {}\n"
    );
    let again = plan(&root, &PwaFlags::default()).unwrap();
    assert!(again.edits.is_empty());
    assert_eq!((again.layouts, again.present), (0, 1));
}

#[test]
fn only_layouts_that_register_the_worker_are_counted() {
    let temporary = tempfile::tempdir().unwrap();
    let root = application(temporary.path());
    // Only the e-mail template has a head: nothing registers the worker.
    fs::write(
        root.join("src/pages/layout.rs"),
        "pub fn page() -> &'static str { \"<p></p>\" }\n",
    )
    .unwrap();
    // Two health merges are ambiguous.
    fs::write(
        root.join("src/routes.rs"),
        format!("{ROUTES}pub fn probes() -> rullst::Router {{\n    rullst::Router::new().merge_axum(rullst::health::health_router())\n}}\n"),
    )
    .unwrap();
    let planned = plan(&root, &PwaFlags::default()).unwrap();
    assert_eq!((planned.layouts, planned.present), (0, 0));
    let report = planned.report();
    assert!(!report.contains("register /sw.js."), "{report}");
    assert!(
        report.contains("To do: No page layout was found in src/"),
        "{report}"
    );
    assert!(
        report
            .contains("not mounted: src/ merges `rullst::health::health_router()` more than once"),
        "{report}"
    );
    assert!(!planned.changed.contains(&"src/routes.rs".to_owned()));
    assert!(
        planned
            .created
            .contains(&"static/manifest.webmanifest".to_owned())
    );
}

#[test]
fn generated_texts_follow_the_language() {
    let temporary = tempfile::tempdir().unwrap();
    let root = application(temporary.path());
    let portuguese = PwaFlags {
        lang: Some("pt-BR".to_owned()),
        ..PwaFlags::default()
    };
    let planned = plan(&root, &portuguese).unwrap();
    assert!(!planned.report().contains("are in English"));
    writes::apply(&planned.edits).unwrap();
    let registration = read(&root, "static/pwa.js");
    assert!(registration.contains(
        "    update: 'Uma nova vers\\u00e3o deste aplicativo est\\u00e1 dispon\\u00edvel.',\n    reload: 'Recarregar',\n"
    ));
    assert!(registration.is_ascii());
    assert!(!registration.contains(".style"));
    let module = read(&root, "src/pwa.rs");
    assert!(module.contains(
        "<p>\"Esta página precisa de conexão. Verifique a sua rede e tente de novo.\"</p>"
    ));
    assert!(module.contains("<a href=\"\">\"Tentar de novo\"</a>"));
    assert_eq!(read(&root, "static/pwa.css"), files::STYLESHEET);
    assert!(files::STYLESHEET.contains(":where(.pwa-update)"));

    // English is the default; a language without texts gets English and a note.
    let english = files::registration(&texts::for_lang("en").0);
    assert_eq!(english, files::REGISTRATION);
    let temporary = tempfile::tempdir().unwrap();
    let root = application(temporary.path());
    let french = PwaFlags {
        lang: Some("fr".to_owned()),
        ..PwaFlags::default()
    };
    let report = plan(&root, &french).unwrap().report();
    assert!(
        report.contains("The offline page and the update notice are in English (make:pwa writes en, pt and es; --lang is fr)"),
        "{report}"
    );
}

#[test]
fn test_and_mail_sources_are_recognized_by_name() {
    for test in [
        "src/tests.rs",
        "src/security_tests.rs",
        "src/pages/layout_test.rs",
        "src/tests/pages.rs",
    ] {
        assert!(is_test_source(test), "{test}");
    }
    for source in ["src/main.rs", "src/pages/contests.rs", "src/testing.rs"] {
        assert!(!is_test_source(source), "{source}");
    }
    for mail in [
        "src/mail/site_mail.rs",
        "src/emails/welcome.rs",
        "src/notifications/booking_email.rs",
        "src/mailer.rs",
        "src/mail_templates.rs",
    ] {
        assert!(is_mail_path(mail), "{mail}");
    }
    for page in ["src/pages/mailbox.rs", "src/main.rs", "src/pages/layout.rs"] {
        assert!(!is_mail_path(page), "{page}");
    }
    assert!(sends_mail("use rullst::mail::Message;"));
    assert!(sends_mail("rullst_mail::Mail::send(m)"));
    assert!(!sends_mail("use rullst::htmx::render_page_with_lang;"));
}

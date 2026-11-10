#![allow(clippy::unwrap_used, clippy::expect_used)]
//! `add alpine`, the layout search, project facts, error reports and the
//! `add` command's frontend values.

use super::layout::{Layout, Search, head_closings, insert_before_head};
use super::project::{has_assets, locked_rullst};
use super::tests::{Harness, LAYOUT, frontend_error, project, read};
use super::{FrontendError, alpine, find, friendly};
use crate::generators::frontend::tailwind::ToolError;
use crate::generators::frontend::{
    ALPINE_COMPONENTS, ALPINE_VENDORED, TAILWIND_INPUT, alpine::integrity, alpine_release::BANNER,
};
use std::fs;

/// A gzip tarball holding the CSP build at its npm path.
fn alpine_tarball(code: &[u8]) -> Vec<u8> {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_size(code.len() as u64);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(0o600);
    header.set_cksum();
    builder
        .append_data(
            &mut header,
            crate::generators::frontend::alpine_release::ENTRY,
            code,
        )
        .unwrap();
    builder.into_inner().unwrap().finish().unwrap()
}

#[test]
fn alpine_vendors_the_verified_build_adds_the_component_and_loads_both() {
    let root = project(&[("src/pages/layout.rs", LAYOUT)]);
    let tgz = alpine_tarball(b"window.Alpine={};");
    let pinned = integrity(&tgz);
    let harness = Harness::serving(&tgz);

    let dry = harness.run(root.path(), "alpine", true, &pinned).unwrap();
    assert_eq!(harness.fetcher.calls.get(), 0);
    assert!(dry.contains("would vendor @alpinejs/csp"), "{dry}");
    assert!(!root.path().join(ALPINE_COMPONENTS).exists());

    let report = harness.run(root.path(), "alpine", false, &pinned).unwrap();
    assert_eq!(
        read(root.path(), ALPINE_VENDORED),
        format!("{BANNER}\nwindow.Alpine={{}};")
    );
    assert_eq!(read(root.path(), ALPINE_COMPONENTS), alpine::COMPONENTS_JS);
    let layout = read(root.path(), "src/pages/layout.rs");
    assert!(
        layout.contains(&format!(
            "                <title>{{title}}</title>\n                {}\n                {}\n            </head>\n",
            alpine::SCRIPT_LINES[0],
            alpine::SCRIPT_LINES[1]
        )),
        "{layout}"
    );
    assert!(report.contains("vendored @alpinejs/csp"), "{report}");
    assert!(
        report.contains(alpine::EXAMPLE.lines().next().unwrap()),
        "{report}"
    );

    let again = harness.run(root.path(), "alpine", false, &pinned).unwrap();
    assert_eq!(harness.fetcher.calls.get(), 1);
    assert!(again.contains("Nothing to change"), "{again}");
    assert!(again.contains("already loads Alpine"), "{again}");
    assert!(again.contains("exists; kept as it is"), "{again}");

    // An older vendored copy is replaced; the edited component is kept.
    fs::write(
        root.path().join(ALPINE_VENDORED),
        "/*! @alpinejs/csp 3.0.0 */\nold",
    )
    .unwrap();
    fs::write(root.path().join(ALPINE_COMPONENTS), "// mine\n").unwrap();
    let replaced = harness.run(root.path(), "alpine", false, &pinned).unwrap();
    assert!(
        replaced.contains("replaced with @alpinejs/csp"),
        "{replaced}"
    );
    assert!(read(root.path(), ALPINE_VENDORED).starts_with(BANNER));
    assert_eq!(read(root.path(), ALPINE_COMPONENTS), "// mine\n");
}

#[test]
fn an_alpine_tarball_without_the_pinned_integrity_writes_nothing() {
    let root = project(&[("src/pages/layout.rs", LAYOUT)]);
    let harness = Harness::serving(&alpine_tarball(b"evil"));
    let error = frontend_error(harness.run(
        root.path(),
        "alpine",
        false,
        crate::generators::frontend::alpine_release::INTEGRITY,
    ));
    assert!(matches!(error, FrontendError::Vendor(_)), "{error}");
    assert!(!root.path().join(ALPINE_VENDORED).exists());
    assert!(!root.path().join(ALPINE_COMPONENTS).exists());
    assert_eq!(read(root.path(), "src/pages/layout.rs"), LAYOUT);
    let friendly = friendly(&error);
    assert_eq!(
        friendly.title,
        "The Alpine.js CSP build could not be vendored"
    );
    assert_eq!(
        friendly.docs,
        Some("https://rullst.github.io/Rullst/book/frontend.html")
    );
}

#[test]
fn head_closings_are_found_only_inside_html_macros() {
    assert_eq!(head_closings(LAYOUT), [11]);
    let nested = "fn a() -> M { rullst::html! { <html>{ html! { <head></head> } }</html> } }";
    assert_eq!(head_closings(nested), [1]);
    for ignored in [
        "fn a() -> &'static str { r#\"<head>\n</head>\"# }",
        "fn a() { other! { <head></head> } }",
        "fn a( {",
        "fn a() { html! { <header></header> } }",
    ] {
        assert!(head_closings(ignored).is_empty(), "{ignored}");
    }
    let layout = Layout {
        path: "src/layout.rs".into(),
        text: "a\r\n    </head>\r\nb\r\n".into(),
        line: 2,
    };
    assert_eq!(
        insert_before_head(&layout, &["<x />", "<y />"]).unwrap(),
        "a\r\n        <x />\r\n        <y />\r\n    </head>\r\nb\r\n"
    );
    let inline = Layout {
        text: "a\n<title>t</title></head>\n".into(),
        ..layout.clone()
    };
    assert_eq!(insert_before_head(&inline, &["<x />"]), None);
    let past_end = Layout { line: 9, ..layout };
    assert_eq!(insert_before_head(&past_end, &["<x />"]), None);
    let root = project(&[]);
    assert_eq!(super::layout::find(root.path()), Search::NotFound);
}

#[test]
fn the_locked_rullst_decides_whether_layouts_can_use_assets() {
    let lock = "version = 4\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"rullst\"\nversion = \"13.0.0-alpha.4\"\n";
    assert_eq!(locked_rullst(lock).as_deref(), Some("13.0.0-alpha.4"));
    assert_eq!(locked_rullst("version = 4\n"), None);
    assert_eq!(locked_rullst("not [toml"), None);
    assert!(!has_assets("13.0.0-alpha.4"));
    assert!(!has_assets("12.3.0"));
    for recent in [
        "13.0.0-alpha.5",
        "13.0.0-rc.1",
        "13.0.0",
        "13.1.0",
        "garbage",
    ] {
        assert!(has_assets(recent), "{recent}");
    }
}

#[test]
fn every_error_has_a_fix_and_the_frontend_docs() {
    let errors = [
        FrontendError::Conflict {
            file: TAILWIND_INPUT,
            why: "x",
            fix: "do y",
        },
        FrontendError::Read {
            file: "a".into(),
            kind: std::io::ErrorKind::PermissionDenied,
        },
        FrontendError::Write {
            file: "a".into(),
            kind: std::io::ErrorKind::PermissionDenied,
        },
        FrontendError::Tool(ToolError::ChecksumMismatch {
            path: "/c/t".into(),
            expected: "e",
            actual: "a".into(),
        }),
        FrontendError::Tool(ToolError::UnsupportedPlatform {
            os: "x".into(),
            arch: "y".into(),
        }),
        FrontendError::Tool(ToolError::NoCacheDirectory),
        FrontendError::Vendor(crate::generators::frontend::alpine::VendorError::MissingEntry),
    ];
    let mut titles = std::collections::BTreeSet::new();
    for error in &errors {
        let friendly = friendly(error);
        assert!(friendly.fix.as_deref().is_some_and(|fix| !fix.is_empty()));
        assert_eq!(friendly.happened, error.to_string());
        titles.insert(friendly.title);
    }
    assert_eq!(titles.len(), errors.len());
    assert_eq!(friendly(&errors[0]).fix.as_deref(), Some("do y"));
    assert!(friendly(&errors[3]).fix.unwrap().contains("Delete /c/t"));
}

#[test]
fn the_command_also_takes_the_frontend_tools() {
    for name in ["tailwind", "alpine", "mail", "studio"] {
        let matches = crate::generators::add::command()
            .try_get_matches_from(["add", name])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("capability").map(String::as_str),
            Some(name)
        );
    }
    let help = crate::generators::add::command()
        .render_long_help()
        .to_string();
    assert!(help.contains("Frontend tools:"), "{help}");
    assert!(help.contains("tailwind Tailwind CSS v4"), "{help}");
    assert!(
        help.contains("alpine   the vendored Alpine.js CSP build"),
        "{help}"
    );
    assert!(help.contains("mail     transactional email"), "{help}");
    for kit in ["tailwind", "alpine"] {
        let hints = crate::generators::add::hints(kit);
        assert!(!hints.is_empty(), "{kit}");
        assert_eq!(hints, find(kit).unwrap().hints);
    }
    assert!(!crate::generators::add::hints("auth").is_empty());
}

#[test]
fn a_frontend_tool_outside_a_project_asks_for_one() {
    // The cargo-rullst package does not depend on `rullst`; a dry run
    // downloads nothing.
    let matches = crate::generators::add::command()
        .try_get_matches_from(["add", "tailwind", "--dry-run"])
        .unwrap();
    let error = crate::generators::add::run(&matches).unwrap_err();
    assert!(
        error
            .downcast_ref::<crate::ui::error_report::ProjectRequired>()
            .is_some(),
        "{error}"
    );
}

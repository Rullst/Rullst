//! The rendered portfolio under long content and, when
//! `RULLST_UI_BROWSER_TESTS=1` (Linux CI), in a real mobile Chromium viewport
//! (`.github/mobile-ui-browser-smoke.mjs`).

use super::*;

/// Checks HTML from the real Rust renderer; the browser run is opt-in.
fn check_mobile_ui(kind: &str, html: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};

    assert!(html.contains("name=\"viewport\""));
    match std::env::var("RULLST_UI_BROWSER_TESTS").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("0") => {
            eprintln!(
                "Rendered HTML checked; real Chromium checks require RULLST_UI_BROWSER_TESTS=1 (enabled in Linux CI)."
            );
            return;
        }
        Ok("1") => {}
        other => panic!("invalid RULLST_UI_BROWSER_TESTS: {other:?}"),
    }
    let script =
        std::env::var_os("RULLST_UI_BROWSER_SCRIPT").expect("explicit browser test script in CI");
    let mut child = Command::new("node")
        .arg(script)
        .arg(kind)
        .stdin(Stdio::piped())
        .spawn()
        .expect("Node 24 and Chromium are required when browser checks are enabled");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(html.as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success(), "{kind} browser regression");
}

#[test]
fn rendered_portfolio_covers_long_content_and_mobile_browser_when_requested() {
    let long = "LongUnbrokenPortfolioContent".repeat(12);
    let profile = Profile {
        id: 1,
        name: long.clone(), title: long.clone(), subtitle: long.clone(),
        email: format!("{long}@example.test"), website: format!("https://example.test/{long}"),
        avatar_url: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='100' height='100'%3E%3C/svg%3E".into(),
        github_url: format!("https://example.test/{long}"),
        linkedin_url: format!("https://example.test/{long}"),
        snapshot: rullst::db::RowSnapshot::default(),
    };
    let project = Project {
        id: 1,
        title: long.clone(),
        description: long.clone(),
        url: "https://example.test/project".into(),
        tags: long.clone(),
        is_featured: 1,
        snapshot: rullst::db::RowSnapshot::default(),
    };
    let experience = Experience {
        id: 1,
        role: long.clone(),
        company: long.clone(),
        period: long.clone(),
        description: long.clone(),
        snapshot: rullst::db::RowSnapshot::default(),
    };
    let skill = Skill {
        id: 1,
        name: long.clone(),
        category: long.clone(),
        snapshot: rullst::db::RowSnapshot::default(),
    };
    let html = render(&profile, &[project], &[experience], &[skill], "test-nonce");
    assert!(html.contains(&long));
    // The production CSP allows only nonce-bound inline styles and same-origin
    // fonts and images (moved from the CLI's former blueprint checks).
    assert!(html.contains("<style nonce=\"test-nonce\">"));
    assert_eq!(
        html.matches("<style").count(),
        html.matches("<style nonce=\"test-nonce\">").count()
    );
    for blocked in [
        "style=\"",
        "fonts.googleapis.com",
        "raw.githubusercontent.com",
    ] {
        assert!(!html.contains(blocked), "{blocked}");
    }
    assert!(html.contains("Projects Showcase"));
    assert!(html.contains("/nexus"));
    check_mobile_ui("portfolio", &html);
}

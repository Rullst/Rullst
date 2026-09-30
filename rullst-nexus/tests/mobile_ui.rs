// Test-only bridge: the caller supplies HTML from the real Rust renderer.
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
    // The page is served with the default production CSP and the real
    // same-origin assets, so a CSP violation or asset error fails the check.
    let mut child = Command::new("node")
        .arg(script)
        .arg(kind)
        .arg(rullst_core::security::DEFAULT_CSP_TEMPLATE)
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
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
fn nexus_shell_renders_and_checks_mobile_browser_when_requested() {
    use rullst_nexus::nexus::{types::*, ui::*};
    use std::sync::Arc;
    let state = NexusState {
        registry: Arc::new(vec![RegistryEntry {
            table: "projects",
            label: "Projects",
            icon: "📁",
            pk: "id",
            tenant_column: None,
            fields: vec![],
        }]),
        brand: Arc::new("Portfolio & CMS".to_owned()),
        audit_policy: NexusAuditPolicy::Disabled,
    };
    let html = render_shell(&state, &render_sidebar(&state, None), "<h1>Portfolio</h1>");
    assert!(html.contains("Portfolio &amp; CMS"));
    assert!(html.contains("/nexus/table/projects"));
    assert!(html.contains("<script src=\"/nexus/assets/nexus.js\" defer></script>"));
    assert!(
        !html.contains("https://"),
        "the shell loads no external resource"
    );
    check_mobile_ui("nexus", &html);
}

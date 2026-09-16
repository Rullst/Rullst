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

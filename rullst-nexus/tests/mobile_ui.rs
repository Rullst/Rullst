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

#[tokio::test]
async fn nexus_shell_renders_and_checks_mobile_browser_when_requested() {
    use rullst_nexus::nexus::{crud::views::render_record_form, types::*, ui::*};
    use std::sync::Arc;
    let entry = RegistryEntry {
        table: "projects",
        label: "Projects",
        icon: "📁",
        pk: "id",
        pk_columns: Vec::new(),
        tenant_column: None,
        fields: vec![FieldMeta::new("title", "Title", FieldKind::Text)],
    };
    let state = NexusState {
        registry: Arc::new(vec![entry.clone()]),
        brand: Arc::new("Portfolio & CMS".to_owned()),
        audit_policy: NexusAuditPolicy::Disabled,
    };
    // A table wider than the screen and the table view's record dialog with
    // the real create form: the browser check saves it, the fixture server
    // answers 404, and the error must be readable above the dialog backdrop.
    let headers: String = (1..=24)
        .map(|column| format!("<th class=\"nexus-th\">Column number {column}</th>"))
        .collect();
    let form = render_record_form(&state, &entry, None, None).await;
    assert!(form.contains("data-nexus-save=\"true\""), "{form}");
    let content = format!(
        "<h1>Portfolio</h1>\
         <div class=\"nexus-table-wrap\"><table class=\"nexus-table\"><thead><tr class=\"nexus-thead-row\">{headers}</tr></thead></table></div>\
         <dialog id=\"nexus-modal\" class=\"nexus-modal\">\
         <button type=\"button\" class=\"nexus-modal-close\" data-nexus-modal-close=\"true\" aria-label=\"Close\">&times;</button>\
         <div class=\"nexus-modal-inner\" id=\"nexus-modal-body\">{form}</div></dialog>"
    );
    let html = render_shell(&state, &render_sidebar(&state, None), &content);
    assert!(html.contains("Portfolio &amp; CMS"));
    assert!(html.contains("/nexus/table/projects"));
    assert!(html.contains("<script src=\"/nexus/assets/nexus.js\" defer></script>"));
    assert!(
        !html.contains("https://"),
        "the shell loads no external resource"
    );
    check_mobile_ui("nexus", &html);
}

#[test]
fn wide_tables_scroll_and_messages_follow_an_open_dialog() {
    use rullst_nexus::nexus::ui::NEXUS_CSS;
    let script = include_str!("../assets/nexus.js");
    let wrap = NEXUS_CSS
        .lines()
        .find(|line| line.starts_with(".nexus-table-wrap {"))
        .expect("table wrapper rule");
    // `overflow: hidden` cut the last columns of a wide table off.
    assert!(wrap.contains("overflow-x: auto;"), "{wrap}");
    assert!(!wrap.contains("overflow: hidden"), "{wrap}");
    // A transformed dialog would be the containing block of the fixed toast.
    let modal = NEXUS_CSS
        .split(".nexus-modal {")
        .nth(1)
        .and_then(|rule| rule.split('}').next())
        .expect("modal rule");
    assert!(!modal.contains("transform"), "{modal}");
    // The toast host moves into an open modal dialog, which the browser
    // draws above the page, and back once it closes.
    assert!(script.contains("document.querySelector(\"dialog[open]\")"));
    assert!(script.contains("dialog.append(host);"));
    assert!(script.contains("document.addEventListener(\"close\""));
    assert!(script.contains("placeToastHost(host);\n"));
}

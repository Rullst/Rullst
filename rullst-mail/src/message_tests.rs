use super::*;

#[test]
fn escapes_text_and_attribute_delimiters() {
    assert_eq!(
        escape_html("<script data-x='\"'>&"),
        "&lt;script data-x=&#x27;&quot;&#x27;&gt;&amp;"
    );
}

#[test]
fn builders_preserve_explicit_text_and_construct_attachments() {
    let raw = Attachment::new("raw.bin", vec![1, 2], "application/octet-stream");
    let message = Message::default()
        .to("reader@example.com")
        .subject("Guide")
        .text("explicit fallback")
        .html("<p>HTML body</p>")
        .from("team@rullst.dev")
        .attach(raw.clone())
        .attach_bytes("guide.txt", b"guide".to_vec(), "text/plain")
        .attach_cid("logo", "logo.png", vec![137, 80], "image/png");

    assert_eq!(message.body_text.as_deref(), Some("explicit fallback"));
    assert_eq!(message.body_html.as_deref(), Some("<p>HTML body</p>"));
    assert_eq!(message.attachments.len(), 3);
    assert_eq!(message.attachments[0], raw);
    assert_eq!(message.attachments[1].filename, "guide.txt");
    assert_eq!(message.attachments[2].cid.as_deref(), Some("logo"));

    let derived = Message::new().html("<h1>Hello</h1><p>Readable</p>");
    assert_eq!(derived.body_text.as_deref(), Some("Hello\n\nReadable"));
}

#[test]
fn scheduling_is_explicit_for_normal_and_unrepresentable_durations() {
    let exact = Utc::now() + chrono::Duration::minutes(5);
    assert_eq!(Message::new().send_at(exact).send_at, Some(exact));

    let soon = Message::new().send_in(std::time::Duration::from_secs(1));
    assert!(soon.send_at.is_some_and(|timestamp| timestamp > Utc::now()));

    let overflow = Message::new().send_in(std::time::Duration::MAX);
    assert_eq!(overflow.send_at, Some(DateTime::<Utc>::MAX_UTC));
}

#[test]
fn attach_file_reads_bytes_and_propagates_io_failures() {
    let path = std::env::temp_dir().join(format!("rullst-mail-message-{}.txt", std::process::id()));
    std::fs::write(&path, b"portable attachment").expect("write fixture");
    let message = Message::new()
        .attach_file(&path)
        .expect("read attachment fixture");
    assert_eq!(message.attachments.len(), 1);
    assert_eq!(message.attachments[0].mime_type, "text/plain");
    assert_eq!(message.attachments[0].content, b"portable attachment");
    std::fs::remove_file(&path).expect("remove fixture");

    assert!(Message::new().attach_file(&path).is_err());
}

#[test]
fn security_and_deliverability_helpers_cover_present_and_absent_bodies() {
    let sanitized = Message::new()
        .subject("token=subject-secret")
        .text("password=text-secret")
        .html("<p>api_key=html-secret</p>")
        .sanitize_secrets();
    assert_eq!(sanitized.subject, "token=[REDACTED]");
    assert_eq!(sanitized.body_text.as_deref(), Some("password=[REDACTED]"));
    assert_eq!(
        sanitized.body_html.as_deref(),
        Some("<p>api_key=[REDACTED]</p>")
    );

    assert!(Message::new().validate_security().is_ok());
    assert!(
        Message::new()
            .html(r#"<a href="javascript:alert(1)">unsafe</a>"#)
            .validate_security()
            .is_err()
    );
    let disposable = Message::new().to("reader@mailinator.com");
    assert!(disposable.is_disposable());
    assert!(disposable.validate_deliverability().is_err());
    assert!(
        Message::new()
            .to("reader@rullst.dev")
            .validate_deliverability()
            .is_ok()
    );
}

#[test]
fn tracking_builders_handle_html_absence_success_and_invalid_configuration() {
    const SECRET: &[u8] = b"rullst-message-tracking-secret-with-diversity";
    let plain = Message::new()
        .to("reader@example.com")
        .try_with_open_tracking("not a tracker URL", b"weak", "campaign")
        .expect("no HTML requires no tracking configuration");
    assert!(plain.body_html.is_none());

    let opened = Message::new()
        .to("reader@example.com")
        .html("<body>Hello</body>")
        .try_with_open_tracking("https://track.rullst.dev/", SECRET, "campaign")
        .expect("open tracking");
    assert!(opened.body_html.unwrap().contains("/track/open/v2."));

    let clicked = Message::new()
        .to("reader@example.com")
        .html(r#"<a href="https://rullst.dev">Guide</a>"#)
        .try_with_click_tracking("https://track.rullst.dev", SECRET)
        .expect("click tracking");
    assert!(clicked.body_html.unwrap().contains("/track/click/v2."));

    assert!(
        Message::new()
            .to("reader@example.com")
            .html("<body>Hello</body>")
            .try_with_open_tracking("https://track.rullst.dev", b"weak", "campaign")
            .is_err()
    );
    assert!(
        Message::new()
            .to("reader@example.com")
            .html(r#"<a href="https://rullst.dev">Guide</a>"#)
            .try_with_click_tracking("not a URL", SECRET)
            .is_err()
    );
}

#[test]
#[allow(deprecated)]
fn legacy_tracking_builders_leave_content_unchanged_on_errors() {
    let html = r#"<a href="https://rullst.dev">Guide</a>"#;
    let open = Message::new()
        .to("reader@example.com")
        .html(html)
        .with_open_tracking("not a URL", b"weak", "campaign");
    assert_eq!(open.body_html.as_deref(), Some(html));

    let click = Message::new()
        .to("reader@example.com")
        .html(html)
        .with_click_tracking("not a URL", b"weak");
    assert_eq!(click.body_html.as_deref(), Some(html));

    assert!(
        Message::new()
            .with_open_tracking("not a URL", b"weak", "campaign")
            .body_html
            .is_none()
    );
    assert!(
        Message::new()
            .with_click_tracking("not a URL", b"weak")
            .body_html
            .is_none()
    );
}

#[test]
fn plain_text_conversion_handles_structural_tags_entities_and_hidden_content() {
    let html = concat!(
        "<style>.hidden { color: red; }</style>",
        "<script>alert('hidden')</script>",
        "<h2>Title &amp; subtitle</h2>",
        "<div>first<br/>second</div>",
        "<table><tr><td>&lt;value&gt;</td></tr></table>",
        "<ul><li>one</li><li>two&nbsp;items</li></ul>",
        "&quot;quoted&quot; &apos;apostrophe&apos; &#39;numeric&#39;"
    );
    let plain = strip_html_to_plain_text(html);
    assert!(!plain.contains("hidden"));
    assert!(plain.contains("Title & subtitle"));
    assert!(plain.contains("first\nsecond"));
    assert!(plain.contains("<value>"));
    assert!(plain.contains("• one"));
    assert!(plain.contains("two items"));
    assert!(plain.contains("\"quoted\" 'apostrophe' 'numeric'"));
}

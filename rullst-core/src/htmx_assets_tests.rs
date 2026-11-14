//! `render_page*` links Core's stylesheet and the htmx client with
//! `rullst::assets::url`: content-hashed while a server serves `static/`,
//! with and without the request's CSP nonce, and plain outside a server.
//! The client is htmx 2, or the htmx 1.9.12 an application created before
//! 13.0.0-alpha.5 vendors when the served `static/` has only that one.

use super::{HTMX_1, HTMX_2, HtmxRequest, render_page, render_page_with_lang};
use crate::assets::{Limits, Manifest, TEST_MANIFEST_LOCK, activate, deactivate};
use crate::html::Markup;
use std::sync::Arc;

fn request(nonce: Option<crate::security::CspNonce>) -> HtmxRequest {
    HtmxRequest {
        is_htmx: false,
        trigger: None,
        target: None,
        prompt: None,
        current_url: None,
        csp_nonce: nonce,
        history_restore: false,
        boosted: false,
    }
}

#[test]
fn the_page_links_hashed_assets_while_static_is_mounted() {
    let _guard = TEST_MANIFEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("rullst.css"), "body{margin:0}").unwrap();
    std::fs::write(root.path().join(HTMX_2.script), "/* htmx */").unwrap();
    // An application that added htmx 2 beside its old client loads htmx 2.
    std::fs::write(root.path().join(HTMX_1.script), "/* htmx 1 */").unwrap();
    activate(Arc::new(Manifest::build(
        root.path(),
        false,
        Limits::DEFAULT,
    )));
    let stylesheet = crate::assets::url("rullst.css");
    let script = crate::assets::url(HTMX_2.script);
    assert!(
        stylesheet.starts_with("/static/rullst.") && stylesheet.len() > "/static/rullst.css".len()
    );
    assert!(script.starts_with("/static/htmx-2.0.11.min.") && script.ends_with(".js"));
    assert_ne!(script, "/static/htmx-2.0.11.min.js");

    let nonce = crate::security::CspNonce::generate();
    let strict =
        render_page_with_lang(&request(Some(nonce.clone())), "en", "T", Markup::default()).0;
    assert!(
        strict.contains(&format!(
            "<link rel=\"stylesheet\" href=\"{stylesheet}\" />"
        )),
        "{strict}"
    );
    assert!(
        strict.contains(&format!(
            "<script nonce=\"{}\" src=\"{script}\"></script>",
            nonce.as_str()
        )),
        "{strict}"
    );
    // The indicator rules of htmx 2 still come with the nonce (#608).
    assert!(strict.contains(&format!(
        "<style nonce=\"{}\">.htmx-indicator{{opacity:0;visibility:hidden}}",
        nonce.as_str()
    )));
    assert!(strict.contains("<meta name=\"htmx-config\""));
    let plain = render_page(&request(None), "T", Markup::default()).0;
    assert!(
        plain.contains(&format!("<script src=\"{script}\"></script>")),
        "{plain}"
    );
    assert!(!plain.contains("/static/rullst.css\""));

    deactivate();
    let outside = render_page(&request(None), "T", Markup::default()).0;
    assert!(outside.contains("<link rel=\"stylesheet\" href=\"/static/rullst.css\" />"));
    assert!(outside.contains("<script src=\"/static/htmx-2.0.11.min.js\"></script>"));
}

#[test]
fn an_application_that_vendors_only_htmx_1_keeps_loading_it() {
    let _guard = TEST_MANIFEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join(HTMX_1.script), "/* htmx 1 */").unwrap();
    activate(Arc::new(Manifest::build(
        root.path(),
        false,
        Limits::DEFAULT,
    )));
    let script = crate::assets::url(HTMX_1.script);
    assert!(script.starts_with("/static/htmx-1.9.12.min.") && script.ends_with(".js"));

    let nonce = crate::security::CspNonce::generate();
    let strict = render_page(&request(Some(nonce.clone())), "T", Markup::default()).0;
    assert!(
        strict.contains(&format!(
            "<script nonce=\"{}\" src=\"{script}\"></script>",
            nonce.as_str()
        )),
        "{strict}"
    );
    assert!(!strict.contains("htmx-2.0.11"), "{strict}");
    // The indicator rules htmx 1.9.12 would insert, with the nonce.
    assert!(
        strict.contains(&format!(
            "<style nonce=\"{}\">{}</style>",
            nonce.as_str(),
            HTMX_1.indicator_styles
        )),
        "{strict}"
    );
    assert!(!strict.contains("visibility"), "{strict}");

    // Without either file (a static directory without htmx) the page names
    // htmx 2, the file new applications ship.
    let empty = tempfile::tempdir().unwrap();
    activate(Arc::new(Manifest::build(
        empty.path(),
        false,
        Limits::DEFAULT,
    )));
    let page = render_page(&request(None), "T", Markup::default()).0;
    assert!(page.contains("<script src=\"/static/htmx-2.0.11.min.js\"></script>"));
    deactivate();
}

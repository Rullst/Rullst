//! Tests for the HTMX request/response helpers and `render_page`.

use super::*;
use axum::http::Request;

#[tokio::test]
async fn test_htmx_request_extractor_empty() {
    let req = Request::builder().body(()).unwrap();
    let (mut parts, _) = req.into_parts();
    let htmx_req = HtmxRequest::from_request_parts(&mut parts, &())
        .await
        .unwrap();

    assert!(!htmx_req.is_htmx);
    assert!(htmx_req.trigger.is_none());
    assert!(htmx_req.target.is_none());
    assert!(htmx_req.prompt.is_none());
    assert!(htmx_req.current_url.is_none());
}

#[tokio::test]
async fn test_htmx_request_extractor_headers() {
    let nonce = crate::security::CspNonce::generate();
    let mut req = Request::builder()
        .header("HX-Request", "true")
        .header("HX-Trigger", "my-btn")
        .header("HX-Target", "content-div")
        .header("HX-Prompt", "hello")
        .header("HX-Current-URL", "http://localhost/home")
        .body(())
        .unwrap();
    req.extensions_mut().insert(nonce.clone());
    let (mut parts, _) = req.into_parts();
    let htmx_req = HtmxRequest::from_request_parts(&mut parts, &())
        .await
        .unwrap();

    assert!(htmx_req.is_htmx);
    assert_eq!(htmx_req.trigger.as_deref(), Some("my-btn"));
    assert_eq!(htmx_req.target.as_deref(), Some("content-div"));
    assert_eq!(htmx_req.prompt.as_deref(), Some("hello"));
    assert_eq!(
        htmx_req.current_url.as_deref(),
        Some("http://localhost/home")
    );
    assert_eq!(htmx_req.csp_nonce.as_ref(), Some(&nonce));
}

#[test]
fn test_htmx_response_builder() {
    let res = HtmxResponse::new("Hello world")
        .trigger("custom-event")
        .redirect("/new-path")
        .refresh();

    assert_eq!(res.content, "Hello world");
    assert_eq!(res.trigger.as_deref(), Some("custom-event"));
    assert_eq!(res.redirect.as_deref(), Some("/new-path"));
    assert!(res.refresh);
}

#[tokio::test]
async fn test_htmx_response_into_response() {
    use axum::response::IntoResponse;
    let res = HtmxResponse::new("Hello world")
        .trigger("my-trigger")
        .redirect("/some-redirect")
        .refresh()
        .into_response();

    let headers = res.headers();
    assert_eq!(headers.get("HX-Trigger").unwrap(), "my-trigger");
    assert_eq!(headers.get("HX-Redirect").unwrap(), "/some-redirect");
    assert_eq!(headers.get("HX-Refresh").unwrap(), "true");
}

#[test]
fn test_htmx_response_refresh_builder() {
    let res = HtmxResponse::new("Hello").refresh();
    assert!(res.refresh);
}

#[tokio::test]
async fn test_htmx_response_refresh_into_response() {
    use axum::response::IntoResponse;
    let res = HtmxResponse::new("Hello world").refresh().into_response();
    let headers = res.headers();
    assert_eq!(headers.get("HX-Refresh").unwrap(), "true");
    assert!(headers.get("HX-Trigger").is_none());
    assert!(headers.get("HX-Redirect").is_none());
}

#[test]
fn test_render_page_helper() {
    let req_htmx = HtmxRequest {
        is_htmx: true,
        trigger: None,
        target: None,
        prompt: None,
        current_url: None,
        csp_nonce: None,
    };
    let req_normal = HtmxRequest {
        is_htmx: false,
        trigger: None,
        target: None,
        prompt: None,
        current_url: None,
        csp_nonce: None,
    };

    // HTMX request -> only the inner content fragment
    let res_htmx = render_page(
        &req_htmx,
        "Title",
        Markup::from_trusted("<div>Fragment</div>"),
    );
    assert_eq!(res_htmx.0, "<div>Fragment</div>");

    // Normal request -> wraps in HTML template
    let res_normal = render_page(
        &req_normal,
        "My Page Title",
        Markup::from_trusted("<div>Body Content</div>"),
    );
    assert!(res_normal.0.contains("<!DOCTYPE html>"));
    assert!(res_normal.0.contains("<title>My Page Title</title>"));
    assert!(res_normal.0.contains("<div>Body Content</div>"));
    assert!(res_normal.0.contains("/static/rullst.css"));
    assert!(res_normal.0.contains("/static/rullst.png"));
    assert!(!res_normal.0.contains("https://cdn.tailwindcss.com"));
    assert!(res_normal.0.contains("/static/htmx-1.9.12.min.js"));
    assert!(!res_normal.0.contains("https://unpkg.com"));
}

#[test]
fn test_render_page_edge_cases() {
    let req_normal = HtmxRequest {
        is_htmx: false,
        trigger: None,
        target: None,
        prompt: None,
        current_url: None,
        csp_nonce: None,
    };

    // Empty content
    let res_empty = render_page(&req_normal, "Empty Title", Markup::default());
    assert!(res_empty.0.contains("<title>Empty Title</title>"));
    assert!(res_empty.0.contains("<body class=\"h-full\">"));

    // Unusual titles
    let res_special = render_page(
        &req_normal,
        "<script>alert(1)</script>",
        crate::html! { "content" },
    );
    assert!(
        res_special
            .0
            .contains("<title>&lt;script&gt;alert(1)&lt;/script&gt;</title>")
    );
}

/// `htmx_header` on a helper thread, so a decoder that stops advancing
/// fails the test at the budget instead of hanging the test binary.
fn header_within_budget(headers: &axum::http::HeaderMap, name: &'static str) -> Option<String> {
    let headers = headers.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(htmx_header(&headers, name));
    });
    receiver
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("decoding returns promptly")
}

fn encoded_prompt(value: &'static str) -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("HX-Prompt", HeaderValue::from_static(value));
    headers.insert(
        "HX-Prompt-URI-AutoEncoded",
        HeaderValue::from_static("true"),
    );
    headers
}

#[tokio::test]
async fn prompt_answers_outside_ascii_are_decoded() {
    let mut latin1 = axum::http::HeaderMap::new();
    latin1.insert(
        "HX-Prompt",
        HeaderValue::from_bytes(b"Jos\xe9 Concei\xe7\xe3o").unwrap(),
    );
    assert_eq!(
        header_within_budget(&latin1, "HX-Prompt").as_deref(),
        Some("José Conceição")
    );

    assert_eq!(
        header_within_budget(
            &encoded_prompt("%E6%97%A5%E6%9C%AC%20%E2%82%AC"),
            "HX-Prompt"
        )
        .as_deref(),
        Some("日本 €")
    );
    assert_eq!(
        header_within_budget(&encoded_prompt("Ol%C3%A1 mundo"), "HX-Prompt").as_deref(),
        Some("Olá mundo")
    );

    let request = Request::builder()
        .header("HX-Prompt", "x%E6%97%A5")
        .header("HX-Prompt-URI-AutoEncoded", "true")
        .header("HX-Target", "plain-id")
        .body(())
        .unwrap();
    let (mut parts, _) = request.into_parts();
    let htmx = HtmxRequest::from_request_parts(&mut parts, &())
        .await
        .unwrap();
    assert_eq!(htmx.prompt.as_deref(), Some("x日"));
    assert_eq!(htmx.target.as_deref(), Some("plain-id"));

    for malformed in ["%E6%9", "%+f", "%FF"] {
        assert_eq!(
            header_within_budget(&encoded_prompt(malformed), "HX-Prompt"),
            None,
            "{malformed}"
        );
    }
}

#[test]
fn page_language_defaults_to_pt_br_and_can_be_declared() {
    let request = HtmxRequest {
        is_htmx: false,
        trigger: None,
        target: None,
        prompt: None,
        current_url: None,
        csp_nonce: None,
    };
    let page = render_page(&request, "Painel", Markup::default());
    assert!(page.0.contains("<html lang=\"pt-BR\""), "{}", page.0);

    let page = render_page_with_lang(&request, "en", "Dashboard", Markup::default());
    assert!(page.0.contains("<html lang=\"en\""), "{}", page.0);

    let page = render_page_with_lang(&request, "\"><script>", "x", Markup::default());
    assert!(!page.0.contains("\"><script>"), "{}", page.0);
}

#[tokio::test]
async fn rendered_htmx_script_uses_the_request_csp_nonce() {
    let nonce = crate::security::CspNonce::generate();
    let mut request = Request::new(());
    request.extensions_mut().insert(nonce.clone());
    let (mut parts, _) = request.into_parts();
    let htmx = HtmxRequest::from_request_parts(&mut parts, &())
        .await
        .unwrap();

    let response = render_page(&htmx, "Nonce", crate::html::RawHtml::new("content"));
    assert!(
        response
            .0
            .contains(&format!("nonce=\"{}\"", nonce.as_str()))
    );
}

#[test]
fn head_markup_ends_the_document_head_and_stays_out_of_htmx_fragments() {
    let page_request = HtmxRequest {
        is_htmx: false,
        trigger: None,
        target: None,
        prompt: None,
        current_url: None,
        csp_nonce: None,
    };
    let head = crate::html! { <link rel="manifest" href="/static/manifest.webmanifest" /> };
    let page = render_page_with_head(
        &page_request,
        "en",
        "Installable",
        head.clone(),
        crate::html! { <p>"body"</p> },
    );
    assert!(
        page.0.contains(
            "<script src=\"/static/htmx-1.9.12.min.js\"></script><link rel=\"manifest\" href=\"/static/manifest.webmanifest\" /></head><body class=\"h-full\"><p>body</p></body>"
        ),
        "{}",
        page.0
    );
    assert!(page.0.contains("<html lang=\"en\""), "{}", page.0);

    let fragment_request = HtmxRequest {
        is_htmx: true,
        ..page_request.clone()
    };
    let fragment = render_page_with_head(
        &fragment_request,
        "en",
        "Installable",
        head,
        crate::html! { <p>"body"</p> },
    );
    assert_eq!(fragment.0, "<p>body</p>");

    // Without head markup the document is the one render_page_with_lang renders.
    let plain = render_page_with_lang(&page_request, "en", "Same", Markup::default());
    let empty_head = render_page_with_head(
        &page_request,
        "en",
        "Same",
        Markup::default(),
        Markup::default(),
    );
    assert_eq!(plain.0, empty_head.0);
    assert!(plain.0.contains("</script></head>"), "{}", plain.0);
}

#[test]
fn nonce_pages_keep_htmx_indicator_rules_inside_the_csp() {
    let nonce = crate::security::CspNonce::generate();
    let request = HtmxRequest {
        is_htmx: false,
        trigger: None,
        target: None,
        prompt: None,
        current_url: None,
        csp_nonce: Some(nonce.clone()),
    };
    let page = render_page(&request, "Indicators", Markup::default()).0;
    // htmx's own nonce-less <style> is turned off; its rules carry the nonce.
    assert!(page.contains(
        "<meta name=\"htmx-config\" content=\"{&quot;includeIndicatorStyles&quot;:false}\" />"
    ));
    let styled = format!(
        "<style nonce=\"{}\">.htmx-indicator{{opacity:0}}",
        nonce.as_str()
    );
    assert!(page.contains(&styled), "{page}");
    assert!(
        page.contains(".htmx-request .htmx-indicator{opacity:1;transition:opacity 200ms ease-in}")
    );
    assert!(
        page.contains(".htmx-request.htmx-indicator{opacity:1;transition:opacity 200ms ease-in}")
    );
    assert_eq!(page.matches("<style").count(), 1);

    // An application's own htmx-config in the head stays the only one.
    let own = crate::html! { <meta name="htmx-config" content="{\"historyCacheSize\":0}" /> };
    let page = render_page_with_head(&request, "en", "Own", own, Markup::default()).0;
    assert_eq!(page.matches("htmx-config").count(), 1, "{page}");
    assert!(page.contains(&styled));

    // Without a nonce (no Rullst CSP) htmx keeps its default behaviour.
    let plain = HtmxRequest {
        csp_nonce: None,
        ..request
    };
    let page = render_page(&plain, "Plain", Markup::default()).0;
    assert!(!page.contains("htmx-config"));
    assert!(!page.contains("<style"));
}

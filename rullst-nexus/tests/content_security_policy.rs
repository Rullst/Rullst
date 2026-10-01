//! The Nexus panel must run under the default production nonce CSP.

mod support;
use support::{authenticated_test_router, local_request};

use axum::body::Body;
use axum::http::StatusCode;
use rullst_nexus::{FieldKind, FieldMeta, Nexus, NexusModel};
use tower::ServiceExt;

struct UserModel;
impl NexusModel for UserModel {
    fn nexus_table() -> &'static str {
        "users"
    }
    fn nexus_label() -> &'static str {
        "Users"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("username", "Username", FieldKind::Text),
            FieldMeta::new("is_active", "Active", FieldKind::Boolean),
        ]
    }
}

struct ComplexModel;
impl NexusModel for ComplexModel {
    fn nexus_table() -> &'static str {
        "complex_records"
    }
    fn nexus_label() -> &'static str {
        "Complex Records"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("title", "Title", FieldKind::Text),
            FieldMeta::new("description", "Description", FieldKind::Textarea),
            FieldMeta::new("email", "Email Address", FieldKind::Email),
            FieldMeta::new("website", "Website URL", FieldKind::Url),
            FieldMeta::new("price", "Price", FieldKind::Number),
            FieldMeta::new("secret", "Password", FieldKind::Password),
            FieldMeta::new("created_date", "Date", FieldKind::Date),
            FieldMeta::new("updated_time", "Timestamp", FieldKind::DateTime),
            FieldMeta::new("is_published", "Published", FieldKind::Boolean),
            FieldMeta::new("metadata", "JSON Metadata", FieldKind::Json),
            FieldMeta::new(
                "status",
                "Status",
                FieldKind::Enum {
                    options: vec!["active", "pending", "archived"],
                },
            ),
        ]
    }
}

type Tag = (String, Vec<(String, String)>);

/// Minimal tag/attribute scanner for Nexus-generated markup. Escaped text never
/// contains a raw `<`, so every `<` starts a tag, closing tag or comment.
fn scan_tags(html: &str) -> Vec<Tag> {
    let mut tags = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        rest = &rest[start + 1..];
        if rest.starts_with('/') || rest.starts_with('!') {
            continue;
        }
        let name_end = rest
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(rest.len());
        let name = rest[..name_end].to_ascii_lowercase();
        let mut cursor = &rest[name_end..];
        let mut attributes = Vec::new();
        loop {
            cursor = cursor.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
            if cursor.is_empty() || cursor.starts_with('>') {
                break;
            }
            let end = cursor
                .find(|c: char| c.is_whitespace() || c == '=' || c == '>')
                .unwrap_or(cursor.len());
            let attribute = cursor[..end].to_ascii_lowercase();
            cursor = &cursor[end..];
            let mut value = String::new();
            if let Some(after) = cursor.strip_prefix('=') {
                if let Some(quote @ ('"' | '\'')) = after.chars().next() {
                    let body = &after[1..];
                    let close = body.find(quote).expect("closed attribute quote");
                    value = body[..close].to_owned();
                    cursor = &body[close + 1..];
                } else {
                    let end = after
                        .find(|c: char| c.is_whitespace() || c == '>')
                        .unwrap_or(after.len());
                    value = after[..end].to_owned();
                    cursor = &after[end..];
                }
            }
            attributes.push((attribute, value));
        }
        tags.push((name, attributes));
        rest = cursor;
    }
    tags
}

fn csp_directive<'a>(policy: &'a str, name: &str) -> &'a str {
    policy
        .split(';')
        .map(str::trim)
        .find_map(|directive| directive.strip_prefix(name))
        .unwrap_or_else(|| panic!("CSP has no {name}: {policy}"))
}

/// Asserts that markup needs nothing beyond `'self'` (and `data:` images)
/// under the default production CSP: no inline script/style blocks, no event
/// handler or `hx-on` attributes, no style attributes and no external URLs.
fn assert_runs_under_default_csp(route: &str, html: &str) {
    for (tag, attributes) in scan_tags(html) {
        assert_ne!(tag, "style", "{route}: inline <style> block");
        for (name, value) in &attributes {
            assert!(
                !name.starts_with("on") && !name.starts_with("hx-on") && name != "style",
                "{route}: <{tag}> carries inline code or style `{name}`"
            );
            if matches!(name.as_str(), "src" | "href") && tag != "a" {
                assert!(
                    value.starts_with("/nexus/assets/") || value.starts_with("data:"),
                    "{route}: <{tag}> loads a non-same-origin resource {value}"
                );
            }
        }
        if tag == "script" {
            assert!(
                attributes.iter().any(|(name, _)| name == "src"),
                "{route}: inline <script> block"
            );
        }
    }
}

/// Stores one complex record so the edit form renders its real widgets. A
/// strict PostgreSQL/MySQL build has no server here, so its edit form reports
/// the failure (still as CSP-clean markup) instead.
async fn edit_form_status() -> StatusCode {
    #[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
    {
        rullst_orm::Orm::init_with_options("sqlite::memory:", 1, 10)
            .await
            .expect("isolated CSP fixture database");
        let pool = rullst_orm::Orm::try_pool().expect("CSP fixture pool");
        for sql in [
            "CREATE TABLE complex_records (id INTEGER PRIMARY KEY, title TEXT, \
             description TEXT, email TEXT, website TEXT, price INTEGER, secret TEXT, \
             created_date TEXT, updated_time TEXT, is_published INTEGER, metadata TEXT, \
             status TEXT)",
            "INSERT INTO complex_records VALUES (1, 'Title', 'First line', \
             'ada@example.com', 'https://example.com', 5, 'stored', '2026-01-01', \
             '2026-01-01T10:00:00', 1, '{}', 'legacy')",
        ] {
            rullst_orm::_sqlx::query(sql)
                .execute(pool)
                .await
                .expect("CSP fixture statement");
        }
        StatusCode::OK
    }
    #[cfg(any(feature = "strict-postgres", feature = "strict-mysql"))]
    {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[tokio::test]
// TM-NEXUS-03: the panel must work under the production nonce CSP.
async fn nexus_pages_run_under_the_default_production_csp() {
    let edit_route = "/table/complex_records/1/edit";
    let edit_status = edit_form_status().await;
    let nexus = Nexus::new()
        .with_brand("CSP Suite")
        .register::<UserModel>()
        .register::<ComplexModel>();
    let app = authenticated_test_router(nexus).layer(axum::middleware::from_fn(
        rullst_core::security::headers_middleware,
    ));

    let full_pages = ["/", "/table/users", "/security", "/telemetry", "/chat"];
    let fragments = [
        "/table/complex_records",
        "/table/complex_records/new",
        edit_route,
        "/table/users/search?q=alice",
    ];
    let mut asset_paths = Vec::new();
    for route in full_pages.iter().chain(fragments.iter()) {
        let mut request = local_request().uri(*route);
        if fragments.contains(route) {
            request = request.header("hx-request", "true");
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).expect("valid request"))
            .await
            .expect("handler executed");
        let expected = if *route == edit_route {
            edit_status
        } else {
            StatusCode::OK
        };
        assert_eq!(response.status(), expected, "{route}");
        let policy = response
            .headers()
            .get(axum::http::header::CONTENT_SECURITY_POLICY)
            .expect("production CSP header")
            .to_str()
            .expect("ASCII CSP")
            .to_owned();
        for directive in ["script-src", "style-src"] {
            let sources = csp_directive(&policy, directive);
            assert!(sources.contains("'self'") && sources.contains("'nonce-"));
            assert!(!sources.contains("unsafe-inline") && !sources.contains("unsafe-eval"));
        }
        let body = axum::body::to_bytes(response.into_body(), 512 * 1024)
            .await
            .expect("bounded page");
        let html = String::from_utf8(body.to_vec()).expect("UTF-8 page");
        assert_runs_under_default_csp(route, &html);
        if full_pages.contains(route) {
            for (tag, attributes) in scan_tags(&html) {
                if matches!(tag.as_str(), "script" | "link" | "img") {
                    asset_paths.extend(attributes.into_iter().filter_map(|(name, value)| {
                        (matches!(name.as_str(), "src" | "href")
                            && value.starts_with("/nexus/assets/"))
                        .then_some(value)
                    }));
                }
            }
        }
    }

    asset_paths.sort();
    asset_paths.dedup();
    assert_eq!(
        asset_paths.len(),
        4,
        "htmx, nexus.js, nexus.css and the Rullst logo"
    );
    for path in asset_paths {
        let response = app
            .clone()
            .oneshot(
                local_request()
                    .uri(path.trim_start_matches("/nexus"))
                    .body(Body::empty())
                    .expect("asset request"),
            )
            .await
            .expect("asset response");
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let content_type = response.headers()[axum::http::header::CONTENT_TYPE]
            .to_str()
            .expect("ASCII content type");
        assert!(
            content_type.starts_with("text/javascript")
                || content_type.starts_with("text/css")
                || content_type == "image/png",
            "{path}: {content_type}"
        );
    }

    let csrf = "nexus_csp_csrf_fixture";
    let answer = app
        .oneshot(
            local_request()
                .method("POST")
                .uri("/chat/query")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Cookie", format!("rullst_csrf={csrf}"))
                .header("X-CSRF-Token", csrf)
                .body(Body::from("message=how+many+users"))
                .expect("chat request"),
        )
        .await
        .expect("chat response");
    assert_eq!(answer.status(), StatusCode::OK);
    let body = axum::body::to_bytes(answer.into_body(), 512 * 1024)
        .await
        .expect("bounded answer");
    let html = String::from_utf8(body.to_vec()).expect("UTF-8 answer");
    assert_runs_under_default_csp("/chat/query", &html);
}

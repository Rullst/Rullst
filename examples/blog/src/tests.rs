//! Router-level tests of the blog showcase.

mod csp;

use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use tower::ServiceExt;

static DATABASE: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

/// Name of the database the compiled ORM pool speaks: "SQLite" or "Any"
/// normally, "PostgreSQL" or "MySQL" when Cargo feature unification (for
/// example `cargo test --workspace --all-features`) enables a strict driver.
fn compiled_pool_database<DB: rullst_orm::sqlx::Database>(
    _: Option<&rullst_orm::sqlx::Pool<DB>>,
) -> &'static str {
    DB::NAME
}

/// Opens one file-backed SQLite database per test process and creates the
/// schema. Returns `false`, so the caller skips, when a strict non-SQLite ORM
/// driver is compiled in and the SQLite showcase database cannot be opened.
#[must_use]
async fn database() -> bool {
    let compiled = compiled_pool_database(None::<&rullst_orm::RullstPool>);
    if !matches!(compiled, "SQLite" | "Any") {
        eprintln!(
            "skipping: the blog showcase needs SQLite, but the ORM is compiled for {compiled}"
        );
        return false;
    }
    DATABASE
        .get_or_init(|| async {
            let path = std::env::temp_dir().join(format!(
                "rullst-blog-example-tests-{}.db",
                std::process::id()
            ));
            let _ = std::fs::remove_file(&path);
            rullst_orm::Orm::init(&format!("sqlite://{}?mode=rwc", path.display()))
                .await
                .expect("test database");
            crate::app::create_schema().await.expect("posts schema");
        })
        .await;
    true
}

fn csrf_post(path: &str, tenant: &str, form: &str) -> Request<Body> {
    let token = rullst::security::generate_csrf_token();
    Request::post(path)
        .header("X-Tenant-ID", tenant)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::COOKIE, format!("rullst_csrf={token}"))
        .body(Body::from(format!("{form}&_token={token}")))
        .expect("form request")
}

async fn get_html(app: &axum::Router, path: &str, tenant: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            Request::get(path)
                .header("X-Tenant-ID", tenant)
                .body(Body::empty())
                .expect("GET request"),
        )
        .await
        .expect("GET response");
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("bounded HTML body");
    (
        status,
        String::from_utf8(body.to_vec()).expect("UTF-8 HTML"),
    )
}

fn test_router() -> rullst::Router {
    let nexus_auth = rullst_nexus::NexusAuthPolicy::basic(
        "integration-fixture-operator",
        "integration-fixture-credential",
    )
    .expect("valid test-only Nexus policy");
    super::router_with_nexus_auth(nexus_auth).expect("blog router")
}

#[tokio::test]
async fn honeypot_button_hits_the_real_deception_middleware() {
    let store = rullst_security::SecurityStore::global();
    let before = store.honeypot_traps_count.load(Ordering::Relaxed);
    let app = test_router().into_axum();
    let mut request = Request::get("/wp-admin")
        .body(Body::empty())
        .expect("honeypot request");
    request.extensions_mut().insert(ConnectInfo(
        "192.0.2.45:4242"
            .parse::<SocketAddr>()
            .expect("test peer address"),
    ));

    let response = app.oneshot(request).await.expect("honeypot response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(store.honeypot_traps_count.load(Ordering::Relaxed) > before);
    let events = store.live_events.lock().expect("security event lock");
    assert!(events.iter().any(|event| {
        event.event_type == "HONEYPOT_TRAP_TRIGGERED"
            && event.client_ip == "192.0.2.45"
            && event.details.contains("/wp-admin")
            && !event.verified_hmac
    }));
}

#[tokio::test]
async fn showcase_forms_render_and_enforce_the_double_submit_csrf_token() {
    if !database().await {
        return;
    }
    // `Server` applies the canonical production baseline around the app. Model
    // that composition here so the example never regresses to two divergent
    // CSRF cookies when it also mounts the middleware explicitly.
    let app = test_router()
        .into_axum()
        .layer(axum::middleware::from_fn(rullst::security::csrf_middleware));

    for path in ["/", "/pricing"] {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).expect("GET request"))
            .await
            .expect("GET response");
        assert_eq!(response.status(), StatusCode::OK);
        let cookies = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .filter(|value| value.starts_with("rullst_csrf="))
            .collect::<Vec<_>>();
        assert_eq!(cookies.len(), 1, "one layer must own CSRF emission");
        let token = cookies[0]
            .split(';')
            .next()
            .and_then(|pair| pair.strip_prefix("rullst_csrf="))
            .expect("CSRF token")
            .to_owned();
        let body = to_bytes(response.into_body(), 512 * 1024)
            .await
            .expect("HTML body");
        let html = std::str::from_utf8(&body).expect("UTF-8 HTML");
        assert!(html.contains(&format!("name=\"_token\" value=\"{token}\"")));
    }

    let rejected = app
        .clone()
        .oneshot(
            Request::post("/posts")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("title=Blocked&body=Missing+token"))
                .expect("missing-token request"),
        )
        .await
        .expect("missing-token response");
    assert_eq!(rejected.status(), StatusCode::FORBIDDEN);

    let token = rullst::security::generate_csrf_token();
    let accepted = app
        .clone()
        .oneshot(
            Request::post("/posts")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, format!("rullst_csrf={token}"))
                .body(Body::from(format!(
                    "title=Accepted&body=Matching+token&_token={token}"
                )))
                .expect("matching-token request"),
        )
        .await
        .expect("matching-token response");
    assert_eq!(accepted.status(), StatusCode::SEE_OTHER);

    let pricing = app
        .clone()
        .oneshot(
            Request::get("/pricing")
                .body(Body::empty())
                .expect("pricing GET request"),
        )
        .await
        .expect("pricing GET response");
    let cookie = pricing
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .expect("pricing CSRF cookie");
    let token = cookie
        .split(';')
        .next()
        .and_then(|pair| pair.strip_prefix("rullst_csrf="))
        .expect("pricing CSRF token")
        .to_owned();
    let pricing_body = to_bytes(pricing.into_body(), 512 * 1024)
        .await
        .expect("pricing HTML body");
    let pricing_html = std::str::from_utf8(&pricing_body).expect("pricing UTF-8 HTML");
    assert!(pricing_html.contains(&format!("name=\"_token\" value=\"{token}\"")));

    let checkout = app
        .oneshot(
            Request::post("/checkout")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::COOKIE, format!("rullst_csrf={token}"))
                .body(Body::from(format!(
                    "provider=stripe&plan=pro_plan&email=fixture%40example.invalid&_token={token}"
                )))
                .expect("offline checkout fixture request"),
        )
        .await
        .expect("offline checkout fixture response");
    assert_eq!(checkout.status(), StatusCode::OK);
    let checkout_body = to_bytes(checkout.into_body(), 512 * 1024)
        .await
        .expect("offline checkout fixture body");
    let checkout_html = std::str::from_utf8(&checkout_body).expect("offline checkout UTF-8 HTML");
    assert!(checkout_html.contains("OFFLINE FIXTURE"));
}

#[tokio::test]
async fn the_unbuilt_wasm_island_demo_is_not_advertised() {
    if !database().await {
        return;
    }
    let app = test_router().into_axum();
    for path in ["/editor", "/wasm-counter"] {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).expect("GET request"))
            .await
            .expect("GET response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
    let response = app
        .oneshot(Request::get("/").body(Body::empty()).expect("GET request"))
        .await
        .expect("GET response");
    let body = to_bytes(response.into_body(), 512 * 1024)
        .await
        .expect("HTML body");
    let html = std::str::from_utf8(&body).expect("UTF-8 HTML");
    assert!(!html.contains("/editor"));
    assert!(!html.contains("Wasm Island"));
}

#[tokio::test]
async fn post_queries_fail_closed_outside_a_tenant_scope() {
    if !database().await {
        return;
    }
    let error = crate::app::Post::all()
        .await
        .expect_err("an unscoped query must not return every tenant's rows");
    assert!(error.to_string().contains("tenant context is required"));
}

#[tokio::test]
async fn posts_and_repository_views_stay_inside_the_selected_tenant() {
    if !database().await {
        return;
    }
    let app = test_router().into_axum();
    let title = format!("Startup-only story {}", std::process::id());

    let created = app
        .clone()
        .oneshot(csrf_post(
            "/posts",
            "tenant-startup",
            &format!(
                "title={}&body=Visible+to+the+startup+tenant",
                title.replace(' ', "+")
            ),
        ))
        .await
        .expect("store response");
    assert_eq!(created.status(), StatusCode::SEE_OTHER);

    for path in ["/", "/posts/repository"] {
        let (status, own) = get_html(&app, path, "tenant-startup").await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert!(own.contains(&title), "{path} must list the tenant's post");

        let (status, other) = get_html(&app, path, "tenant-enterprise").await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert!(
            !other.contains(&title),
            "{path} leaked another tenant's post"
        );
    }

    let (status, _) = get_html(&app, "/", "tenant-outside-membership").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn seeding_is_idempotent_and_never_deletes_visitor_posts() {
    use crate::app::{PATTERNS_TITLE, Post, SEED_TENANT, WELCOME_TITLE, seed_showcase_posts};
    use rullst_orm::with_tenant;

    if !database().await {
        return;
    }
    // Matches the title and body patterns the old startup cleanup deleted.
    let lookalike = format!(
        "Architecture Deep Dive: our migration {}",
        std::process::id()
    );
    with_tenant("tenant-enterprise", async {
        Post {
            id: 0,
            tenant_id: "tenant-enterprise".to_string(),
            title: lookalike.clone(),
            body: "Notes about Topcoat".to_string(),
        }
        .save()
        .await
    })
    .await
    .expect("visitor post");

    seed_showcase_posts().await.expect("first start");
    seed_showcase_posts().await.expect("restart");

    for title in [WELCOME_TITLE, PATTERNS_TITLE] {
        let seeded = with_tenant(SEED_TENANT, async {
            Post::query().where_eq("title", title).count().await
        })
        .await
        .expect("seed count");
        assert_eq!(seeded, 1, "{title} must be seeded exactly once");
    }
    let kept = with_tenant("tenant-enterprise", async {
        Post::query()
            .where_eq("title", lookalike.as_str())
            .count()
            .await
    })
    .await
    .expect("visitor post count");
    assert_eq!(kept, 1);
}

#[tokio::test]
async fn story_submissions_are_bounded() {
    if !database().await {
        return;
    }
    let app = test_router().into_axum();
    let long_title = "t".repeat(crate::app::MAX_TITLE_CHARS + 1);
    let long_body = "b".repeat(crate::app::MAX_BODY_CHARS + 1);
    for form in [
        format!("title={long_title}&body=ok"),
        format!("title=ok&body={long_body}"),
        "title=+&body=ok".to_string(),
    ] {
        let response = app
            .clone()
            .oneshot(csrf_post("/posts", "tenant-enterprise", &form))
            .await
            .expect("store response");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    let oversized = format!("title=big&body={}", "b".repeat(crate::app::MAX_FORM_BYTES));
    let response = app
        .oneshot(csrf_post("/posts", "tenant-enterprise", &oversized))
        .await
        .expect("oversized response");
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn tenants_keep_a_bounded_number_of_stories_and_listings() {
    use crate::app::{CreatePostError, LISTED_POSTS, MAX_POSTS_PER_TENANT};
    use crate::repository_demo::{BODY_PREVIEW_CHARS, PostRepository, REPOSITORY_ROWS};

    if !database().await {
        return;
    }
    let tenant = format!("quota-{}", std::process::id());
    let body = "x".repeat(crate::app::MAX_BODY_CHARS);
    for number in 0..MAX_POSTS_PER_TENANT {
        crate::app::create_post(&tenant, &format!("Story {number}"), &body)
            .await
            .expect("story within the quota");
    }
    assert!(matches!(
        crate::app::create_post(&tenant, "One too many", "body").await,
        Err(CreatePostError::QuotaReached)
    ));

    let listed = crate::app::recent_posts(&tenant).await.expect("listing");
    assert_eq!(listed.len(), LISTED_POSTS);
    assert_eq!(
        listed[0].title,
        format!("Story {}", MAX_POSTS_PER_TENANT - 1)
    );

    let rows = PostRepository::get_tenant_posts(&tenant)
        .await
        .expect("repository listing");
    assert_eq!(rows.len() as i64, REPOSITORY_ROWS);
    assert!(
        rows.iter()
            .all(|row| row.body.chars().count() as i64 == BODY_PREVIEW_CHARS)
    );
}

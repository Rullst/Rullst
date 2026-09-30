//! Browser- and key-shaped CRUD contracts against a real SQLite database.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

mod support;

use axum::{body::Body, http::StatusCode};
use rullst_nexus::{
    FieldKind, FieldMeta, Nexus, NexusModel, create_nexus_audit_table, recent_nexus_audits,
};
use rullst_orm::{_sqlx as sqlx, Orm, RullstPool};
use support::{authenticated_test_router, local_request};
use tower::ServiceExt;

/// A slug-keyed model: record keys are arbitrary text.
struct Page;

impl NexusModel for Page {
    fn nexus_table() -> &'static str {
        "nexus_pages"
    }

    fn nexus_label() -> &'static str {
        "Pages"
    }

    fn nexus_pk() -> &'static str {
        "slug"
    }

    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("slug", "Slug", FieldKind::Text),
            FieldMeta::new("title", "Title", FieldKind::Text),
        ]
    }
}

/// An integer-keyed model.
struct Counter;

impl NexusModel for Counter {
    fn nexus_table() -> &'static str {
        "nexus_counters"
    }

    fn nexus_label() -> &'static str {
        "Counters"
    }

    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("label", "Label", FieldKind::Text),
        ]
    }
}

/// Nullable number, Boolean and relation columns with a text key.
struct Metric;

impl NexusModel for Metric {
    fn nexus_table() -> &'static str {
        "nexus_metrics"
    }

    fn nexus_label() -> &'static str {
        "Metrics"
    }

    fn nexus_pk() -> &'static str {
        "code"
    }

    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("code", "Code", FieldKind::Text),
            FieldMeta::new("amount", "Amount", FieldKind::Number),
            FieldMeta::new("approved", "Approved", FieldKind::Boolean),
            FieldMeta::new(
                "owner_id",
                "Owner",
                FieldKind::ForeignKey {
                    table: "owners",
                    label_col: "name",
                },
            ),
        ]
    }
}

const CSRF: &str = "crud_contract_csrf_fixture";

async fn body_text(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), 512 * 1024)
        .await
        .expect("bounded response body");
    String::from_utf8(bytes.to_vec()).expect("UTF-8 response")
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            local_request()
                .uri(uri)
                .body(Body::empty())
                .expect("valid GET request"),
        )
        .await
        .expect("GET response");
    (response.status(), body_text(response).await)
}

async fn mutate(app: &axum::Router, method: &str, uri: &str, body: &str) -> StatusCode {
    app.clone()
        .oneshot(
            local_request()
                .method(method)
                .uri(uri)
                .header("content-type", "application/x-www-form-urlencoded")
                .header("cookie", format!("rullst_csrf={CSRF}"))
                .header("x-csrf-token", CSRF)
                .body(Body::from(body.to_owned()))
                .expect("valid mutation request"),
        )
        .await
        .expect("mutation response")
        .status()
}

async fn audited_keys(table: &str, action: &str) -> Vec<Option<String>> {
    recent_nexus_audits(100, None)
        .await
        .expect("load audit records")
        .into_iter()
        .filter(|audit| audit.table_name == table && audit.action == action)
        .map(|audit| audit.record_key)
        .collect()
}

#[tokio::test]
async fn crud_contracts_hold_on_sqlite() {
    Orm::init_with_options("sqlite::memory:", 1, 10)
        .await
        .expect("isolated single-connection database");
    let pool = Orm::try_pool().expect("initialized test pool");
    for ddl in [
        "CREATE TABLE nexus_pages (slug TEXT PRIMARY KEY, title TEXT NOT NULL)",
        "CREATE TABLE nexus_counters (id INTEGER PRIMARY KEY, label TEXT NOT NULL)",
        "CREATE TABLE nexus_metrics (code TEXT PRIMARY KEY, amount INTEGER, \
         approved INTEGER, owner_id INTEGER)",
    ] {
        sqlx::query(ddl)
            .execute(pool)
            .await
            .expect("create fixture table");
    }
    create_nexus_audit_table()
        .await
        .expect("install Nexus audit schema");
    let app = authenticated_test_router(
        Nexus::new()
            .register::<Page>()
            .register::<Counter>()
            .register::<Metric>()
            .with_required_audit(),
    );

    search_matches_wildcards_literally(&app, pool).await;
    numeric_keys_are_canonical_in_sql_and_audit(&app, pool).await;
    unrepresentable_keys_do_not_block_required_audit(&app, pool).await;
    null_and_unreadable_list_values_are_not_fabricated(&app, pool).await;
    browser_forms_carry_the_csrf_body_token(&app, pool).await;
}

/// The bulk-action form is a plain browser POST without the `X-CSRF-Token`
/// header, so it must carry the rendered `_token` field (NEXUS-06), which
/// create also accepts (NX2-10).
async fn browser_forms_carry_the_csrf_body_token(app: &axum::Router, pool: &RullstPool) {
    let response = app
        .clone()
        .oneshot(
            local_request()
                .uri("/table/nexus_counters")
                .body(Body::empty())
                .expect("valid list request"),
        )
        .await
        .expect("list response");
    let cookie = response
        .headers()
        .get("set-cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .expect("CSRF cookie issued to a new browser")
        .to_owned();
    let token = cookie
        .strip_prefix("rullst_csrf=")
        .expect("rullst_csrf cookie")
        .to_owned();
    let html = body_text(response).await;
    let form = html
        .split("<form id=\"batch-form-nexus_counters\"")
        .nth(1)
        .and_then(|rest| rest.split("</form>").next())
        .expect("bulk-action form");
    assert!(
        form.contains(&format!(
            "<input type=\"hidden\" name=\"_token\" value=\"{token}\" />"
        )),
        "the bulk form must carry the request's CSRF token"
    );

    let browser_post = |uri: &'static str, body: String| {
        local_request()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/x-www-form-urlencoded")
            .header("cookie", cookie.as_str())
            .body(Body::from(body))
            .expect("valid browser form POST")
    };
    let created = app
        .clone()
        .oneshot(browser_post(
            "/table/nexus_counters",
            format!("_token={token}&label=via-body-token"),
        ))
        .await
        .expect("create response");
    assert_eq!(created.status(), StatusCode::OK);
    let (id,): (i64,) =
        sqlx::query_as("SELECT id FROM nexus_counters WHERE label = 'via-body-token'")
            .fetch_one(pool)
            .await
            .expect("created through the body token");

    let batch = app
        .clone()
        .oneshot(browser_post(
            "/table/nexus_counters/batch",
            format!("_token={token}&action=delete&selected_ids={id}"),
        ))
        .await
        .expect("batch response");
    assert_eq!(batch.status(), StatusCode::SEE_OTHER);
    let (remaining,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM nexus_counters WHERE label = 'via-body-token'")
            .fetch_one(pool)
            .await
            .expect("count remaining rows");
    assert_eq!(remaining, 0);
}

/// NULL and undecodable numbers, Booleans and relations are not shown as `0`
/// or `No`, and a row without a usable key gets no actions (NX2-06).
async fn null_and_unreadable_list_values_are_not_fabricated(app: &axum::Router, pool: &RullstPool) {
    // SQLite keeps text in INTEGER-affinity columns and allows a NULL text key.
    sqlx::query(
        "INSERT INTO nexus_metrics (code, amount, approved, owner_id) VALUES \
         ('m1', NULL, NULL, NULL), ('m2', 'n/a', 'maybe', 'x'), (NULL, 5, 1, 7)",
    )
    .execute(pool)
    .await
    .expect("insert metric fixtures");
    let (status, html) = get(app, "/table/nexus_metrics/search?q=").await;
    assert_eq!(status, StatusCode::OK);

    let null = "<td class=\"nexus-td nexus-muted\">NULL</td>";
    let unreadable = "<td class=\"nexus-td nexus-muted\">unreadable</td>";
    assert_eq!(html.matches(null).count(), 3, "{html}");
    assert_eq!(html.matches(unreadable).count(), 3, "{html}");
    assert!(!html.contains("<td class=\"nexus-td\">0</td>"), "{html}");
    assert!(!html.contains("No</td>"), "{html}");

    assert_eq!(html.matches("data-nexus-row-id=").count(), 2, "{html}");
    assert!(!html.contains("data-nexus-row-id=\"0\""));
    assert!(html.contains("No usable key"));
    assert_eq!(html.matches("nexus-batch-check").count(), 2);
}

/// A numeric key spelled `+1`, `01` or `1e3` must not change record 1 or
/// 1000, and the audit names the canonical key (NEXUS-09).
async fn numeric_keys_are_canonical_in_sql_and_audit(app: &axum::Router, pool: &RullstPool) {
    sqlx::query("INSERT INTO nexus_counters (id, label) VALUES (1, 'one'), (1000, 'thousand')")
        .execute(pool)
        .await
        .expect("insert counters");

    for spelling in ["+1", "01"] {
        let uri = format!("/table/nexus_counters/{spelling}");
        assert_eq!(
            mutate(app, "PUT", &uri, "label=renamed").await,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        mutate(app, "DELETE", "/table/nexus_counters/1e3", "").await,
        StatusCode::NOT_FOUND
    );
    let (labels,): (String,) = sqlx::query_as(
        "SELECT group_concat(label, ',') FROM (SELECT label FROM nexus_counters ORDER BY id)",
    )
    .fetch_one(pool)
    .await
    .expect("read counters");
    assert_eq!(labels, "one,thousand");

    assert_eq!(
        mutate(app, "PUT", "/table/nexus_counters/1", "label=first").await,
        StatusCode::OK
    );
    assert_eq!(
        mutate(app, "DELETE", "/table/nexus_counters/1000", "").await,
        StatusCode::OK
    );
    // The edit form of a missing or misspelled key is a 404, not an empty
    // editable form (NX2-05).
    for key in ["999", "+1", "01"] {
        let (status, body) = get(app, &format!("/table/nexus_counters/{key}/edit")).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{key}: {body}");
        assert!(!body.contains("<form"));
    }
    let (status, form) = get(app, "/table/nexus_counters/1/edit").await;
    assert_eq!(status, StatusCode::OK);
    assert!(form.contains("value=\"first\""), "{form}");

    assert_eq!(
        audited_keys("nexus_counters", "update").await,
        [Some("1".to_owned())]
    );
    assert_eq!(
        audited_keys("nexus_counters", "delete").await,
        [Some("1000".to_owned())]
    );
}

/// A real key that the audit text format cannot hold is audited as absent;
/// the change commits instead of failing as "audit unavailable" (2P-10).
async fn unrepresentable_keys_do_not_block_required_audit(app: &axum::Router, pool: &RullstPool) {
    let long = "k".repeat(300);
    for key in ["ACME ", long.as_str()] {
        sqlx::query("INSERT INTO nexus_pages (slug, title) VALUES (?, 'before')")
            .bind(key)
            .execute(pool)
            .await
            .expect("insert unusual key");
        let uri = format!("/table/nexus_pages/{}", urlencoding::encode(key));
        assert_eq!(
            mutate(app, "PUT", &uri, "title=after").await,
            StatusCode::OK
        );
        assert_eq!(mutate(app, "DELETE", &uri, "").await, StatusCode::OK);
    }
    assert_eq!(audited_keys("nexus_pages", "update").await, [None, None]);
    assert_eq!(audited_keys("nexus_pages", "delete").await, [None, None]);
}

/// `%` and `_` typed into the search box are literal characters (NX2-11).
async fn search_matches_wildcards_literally(app: &axum::Router, pool: &RullstPool) {
    sqlx::query(
        "INSERT INTO nexus_pages (slug, title) VALUES \
         ('sale', '50% off'), ('stock', '500 units'), \
         ('under', 'a_b'), ('plain', 'axb'), ('person', 'Alice')",
    )
    .execute(pool)
    .await
    .expect("insert search fixtures");

    let (status, html) = get(app, "/table/nexus_pages/search?q=50%25").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("50% off"));
    assert!(!html.contains("500 units"));

    let (_, html) = get(app, "/table/nexus_pages/search?q=a_b").await;
    assert!(html.contains("a_b"));
    assert!(!html.contains("axb"));

    let (_, html) = get(app, "/table/nexus_pages/search?q=alice").await;
    assert!(html.contains("Alice"));

    sqlx::query("DELETE FROM nexus_pages")
        .execute(pool)
        .await
        .expect("reset search fixtures");
}

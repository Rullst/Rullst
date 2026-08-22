//! Studio Data Browser Tests

use super::db::*;

#[test]
fn test_escape_html_attr() {
    let input = r#"<script>alert("XSS & Hack")</script> 'test'"#;
    let expected =
        "&lt;script&gt;alert(&quot;XSS &amp; Hack&quot;)&lt;/script&gt; &#x27;test&#x27;";
    assert_eq!(escape_html_attr(input), expected);
}

#[test]
fn test_sanitize_identifier() {
    assert_eq!(sanitize_identifier("valid_table_123"), "valid_table_123");
    assert_eq!(sanitize_identifier("invalid-table!@#"), "invalidtable");
    assert_eq!(sanitize_identifier("drop table users;--"), "droptableusers");

    let long_id = "a".repeat(100);
    assert_eq!(sanitize_identifier(&long_id).len(), 64);
}

#[test]
fn test_build_headers_html() {
    let cols = vec!["id".to_string(), "name".to_string()];
    let pks = vec![0];
    let html = build_headers_html(&cols, &pks);

    assert!(html.contains("id"));
    assert!(html.contains("name"));
    assert!(html.contains("PK"));

    let cols2 = vec!["created_at".to_string()];
    let html2 = build_headers_html(&cols2, &[]);
    assert!(html2.contains("created_at"));
    assert!(!html2.contains("PK"));
}

#[tokio::test]
#[cfg(not(miri))]
#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
async fn test_db_operations() {
    let pool = ensure_pool_initialized()
        .await
        .expect("pool should be initialized");

    let _ = sqlx::query("DROP TABLE IF EXISTS test_users")
        .execute(pool)
        .await;
    let _ = sqlx::query("DROP TABLE IF EXISTS test_posts")
        .execute(pool)
        .await;

    sqlx::query("CREATE TABLE test_users (id INTEGER PRIMARY KEY, name TEXT);")
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("CREATE TABLE test_posts (id INTEGER PRIMARY KEY, title TEXT);")
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO test_users (name) VALUES ('Alice'), ('Bob')")
        .execute(pool)
        .await
        .unwrap();

    let tables = fetch_tables().await.unwrap();
    assert!(tables.contains(&"test_users".to_string()));
    assert!(tables.contains(&"test_posts".to_string()));

    let users_count = count_table_rows("test_users", None).await.unwrap();
    assert_eq!(users_count, 2);

    let search_count = count_table_rows("test_users", Some("Alice")).await.unwrap();
    assert_eq!(search_count, 1);
}

#[tokio::test]
#[cfg(not(miri))]
#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
async fn test_get_any_value_as_string() {
    let pool = ensure_pool_initialized()
        .await
        .expect("pool should be initialized");

    let row = sqlx::query("SELECT 'hello' as s, 42 as i, 3.14 as f, NULL as n")
        .fetch_one(pool)
        .await
        .unwrap();

    assert_eq!(get_any_value_as_string(&row, 0), "hello");
    assert_eq!(get_any_value_as_string(&row, 1), "42");
    assert_eq!(get_any_value_as_string(&row, 2), "3.14");
    assert_eq!(get_any_value_as_string(&row, 3), "NULL");
}

#[test]
fn test_build_search_clause() {
    assert_eq!(
        build_search_clause("postgres", "col"),
        "CAST(\"col\" AS TEXT) ILIKE "
    );
    assert_eq!(
        build_search_clause("mysql", "col"),
        "CAST(`col` AS CHAR) LIKE "
    );
    assert_eq!(build_search_clause("sqlite", "col"), "\"col\" LIKE ");
}

#[test]
fn test_query_builders() {
    assert!(build_fetch_tables_query("postgres").contains("information_schema"));
    assert!(build_fetch_tables_query("mysql").contains("DATABASE()"));
    assert!(build_fetch_tables_query("sqlite").contains("sqlite_master"));

    assert_eq!(quote_table_name("mysql", "tbl"), "`tbl`");
    assert_eq!(quote_table_name("postgres", "tbl"), "\"tbl\"");

    assert!(build_schema_query("postgres", "tbl").contains("information_schema"));
    assert!(build_schema_query("mysql", "tbl").contains("DATABASE()"));
    assert!(build_schema_query("sqlite", "tbl").contains("PRAGMA"));
}

#[tokio::test]
#[cfg(not(miri))]
#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
async fn test_build_rows_html() {
    let pool = ensure_pool_initialized()
        .await
        .expect("pool should be initialized");

    let row = sqlx::query("SELECT 'hello' as s, NULL as n")
        .fetch_one(pool)
        .await
        .unwrap();

    let html = build_rows_html(&[row], &["s".to_string(), "n".to_string()]);

    assert!(html.contains("text-slate-600 font-mono italic"));
    assert!(html.contains("text-slate-300"));
}

#[tokio::test]
async fn test_studio_layout_and_telemetry_handlers() {
    use super::handlers::*;
    use super::layout::*;
    use axum::http::HeaderMap;

    // Test layout rendering
    let sidebar_empty = render_sidebar_oob(&[], None);
    assert!(sidebar_empty.contains("hidden"));

    let sidebar = render_sidebar_oob(&["users".to_string(), "orders".to_string()], Some("users"));
    assert!(sidebar.contains("Database Schema"));
    assert!(sidebar.contains("users"));
    assert!(sidebar.contains("orders"));

    let full_page = studio_layout(
        "<div>Main Content</div>".to_string(),
        Some("users"),
        &["users".to_string()],
    );
    assert!(full_page.contains("Main Content"));
    assert!(full_page.contains("Rullst Studio"));

    // Test handlers with and without HTMX headers
    let mut htmx_headers = HeaderMap::new();
    htmx_headers.insert("hx-request", "true".parse().unwrap());
    let plain_headers = HeaderMap::new();

    let _ = handle_studio_radar(htmx_headers.clone()).await;
    let _ = handle_studio_radar(plain_headers.clone()).await;

    let _ = handle_studio_capital(htmx_headers.clone()).await;
    let _ = handle_studio_capital(plain_headers.clone()).await;

    let _ = handle_studio_traces(htmx_headers.clone()).await;
    let _ = handle_studio_traces(plain_headers.clone()).await;

    let _ = handle_studio_tools_ai(htmx_headers.clone()).await;
    let _ = handle_studio_tools_ai(plain_headers.clone()).await;

    let _ = handle_studio_tools_migrations(htmx_headers.clone()).await;
    let _ = handle_studio_tools_migrations(plain_headers.clone()).await;

    let _ = handle_studio_tools_security(htmx_headers.clone()).await;
    let _ = handle_studio_tools_security(plain_headers.clone()).await;

    let _ = handle_dashboard(htmx_headers).await;
    let _ = handle_dashboard(plain_headers).await;
}

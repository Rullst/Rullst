use axum::{
    body::{Body, to_bytes},
    extract::{ConnectInfo, Request as AxumRequest},
    http::{Request, StatusCode, header},
    middleware::Next,
    response::Response,
};
use rullst_orm::_sqlx::{self, Database, Execute, QueryBuilder, query::Query};
use rullst_studio::{LocalStudioAccess, Studio};
use std::net::SocketAddr;
use tower::ServiceExt;

mod bounded_view;
mod feature_flags;
mod incomplete_key;

// The SQLite matrix binary under the default build compiles this unused.
#[cfg(not(feature = "strict-sqlite"))]
#[allow(dead_code)]
pub fn handle_container_start_error(provider: &str, error: impl std::fmt::Display) {
    if std::env::var("RULLST_REQUIRE_TESTCONTAINERS").as_deref() == Ok("true") {
        panic!("{provider} testcontainer is required but failed to start: {error}");
    }
    eprintln!("skipping {provider} Studio mutation matrix: {error}");
}

type FixtureQuery<'q> =
    Query<'q, rullst_orm::RullstDatabase, <rullst_orm::RullstDatabase as Database>::Arguments>;

/// Builds a fixture statement for the matrix backend. Under the default
/// `sqlx::Any` build, `QueryBuilder` writes `?` markers that PostgreSQL rejects,
/// so they are renumbered in textual order, which is also the binding order.
pub(crate) fn fixture<'q>(
    builder: &'q mut QueryBuilder<rullst_orm::RullstDatabase>,
    driver: &str,
) -> FixtureQuery<'q> {
    if driver != "postgres" {
        return builder.build();
    }
    let sql = rullst_orm::replace_placeholders(builder.sql().as_str());
    let arguments = builder
        .build()
        .take_arguments()
        .expect("encodable fixture arguments")
        .unwrap_or_default();
    _sqlx::query_with(_sqlx::AssertSqlSafe(sql), arguments)
}

async fn inject_loopback(mut request: AxumRequest, next: Next) -> axum::response::Response {
    request.headers_mut().insert(
        header::HOST,
        axum::http::HeaderValue::from_static("127.0.0.1:5555"),
    );
    request.headers_mut().insert(
        header::ORIGIN,
        axum::http::HeaderValue::from_static("http://127.0.0.1:5555"),
    );
    request.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:42000"
            .parse::<SocketAddr>()
            .expect("loopback test peer"),
    ));
    next.run(request).await
}

async fn response_text(response: Response) -> String {
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .expect("bounded Studio response body");
    String::from_utf8(bytes.to_vec()).expect("Studio renders UTF-8")
}

pub async fn exercise_mutations(database_url: &str, driver: &str, table: &str) {
    assert!(
        !table.is_empty()
            && table.len() <= 57
            && table
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        "matrix table identifier must stay inside the audited allowlist"
    );
    if driver == "sqlite" {
        rullst_orm::Orm::init_with_options(database_url, 1, 5)
            .await
            .expect("Studio SQLite matrix ORM initialization");
    } else {
        rullst_orm::Orm::init(database_url)
            .await
            .expect("Studio matrix ORM initialization");
    }
    let pool = rullst_core::db::safe_pool().expect("Studio matrix pool");
    let quoted_table = if driver == "mysql" {
        format!("`{table}`")
    } else {
        format!("\"{table}\"")
    };
    let parent_table = format!("{table}_parent");
    let quoted_parent = if driver == "mysql" {
        format!("`{parent_table}`")
    } else {
        format!("\"{parent_table}\"")
    };
    let mut create_parent = QueryBuilder::<rullst_orm::RullstDatabase>::new("CREATE TABLE ");
    create_parent
        .push(&quoted_parent)
        .push(" (id BIGINT PRIMARY KEY)");
    fixture(&mut create_parent, driver)
        .execute(pool)
        .await
        .expect("create Studio matrix parent table");
    let mut insert_parent = QueryBuilder::<rullst_orm::RullstDatabase>::new("INSERT INTO ");
    insert_parent
        .push(&quoted_parent)
        .push(" (id) VALUES (")
        .push_bind(1_i64)
        .push(")");
    fixture(&mut insert_parent, driver)
        .execute(pool)
        .await
        .expect("insert Studio matrix parent row");
    let mut create = QueryBuilder::<rullst_orm::RullstDatabase>::new("CREATE TABLE ");
    create.push(&quoted_table).push(
        " (id BIGINT PRIMARY KEY, name VARCHAR(255) NOT NULL, active BOOLEAN NOT NULL, \
         score DOUBLE PRECISION NULL, parent_id BIGINT NOT NULL, FOREIGN KEY (parent_id) \
         REFERENCES ",
    );
    create.push(&quoted_parent).push(" (id))");
    fixture(&mut create, driver)
        .execute(pool)
        .await
        .expect("create Studio matrix table");
    let mut insert = QueryBuilder::<rullst_orm::RullstDatabase>::new("INSERT INTO ");
    insert
        .push(&quoted_table)
        .push(" (id, name, active, score, parent_id) VALUES (")
        .push_bind(7_i64)
        .push(", ")
        .push_bind("before")
        .push(", ")
        .push_bind(true)
        .push(", ")
        .push_bind(1.5_f64)
        .push(", ")
        .push_bind(1_i64)
        .push(")");
    fixture(&mut insert, driver)
        .execute(pool)
        .await
        .expect("insert Studio matrix row");

    let app = Studio::new()
        .into_router(LocalStudioAccess::loopback_only())
        .expect("debug Studio router")
        .layer(axum::middleware::from_fn(inject_loopback));
    let form_request = |suffix: &str, body: &str| {
        Request::builder()
            .method("POST")
            .uri(format!("/studio/tables/{table}/rows/{suffix}"))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body.to_string()))
            .expect("valid Studio matrix request")
    };

    let dashboard = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/studio")
                .body(Body::empty())
                .expect("valid Studio dashboard request"),
        )
        .await
        .expect("Studio dashboard response");
    assert_eq!(dashboard.status(), StatusCode::OK);
    let dashboard = response_text(dashboard).await;
    assert!(dashboard.contains("Rullst Studio Control Center"));
    assert!(
        dashboard.contains(table),
        "dashboard omitted matrix table: {dashboard}"
    );
    assert!(dashboard.contains(&parent_table));

    let migrations = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/studio/migrations")
                .header("hx-request", "true")
                .body(Body::empty())
                .expect("valid Studio migrations request"),
        )
        .await
        .expect("Studio migrations response");
    assert_eq!(migrations.status(), StatusCode::OK);
    let migrations = response_text(migrations).await;
    assert!(migrations.contains(table));
    assert!(migrations.contains("Database schema tools"));
    assert!(migrations.contains("cargo rullst db:migrate"));
    assert!(!migrations.contains("<!DOCTYPE html>"));

    let er_diagram = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/studio/er")
                .body(Body::empty())
                .expect("valid Studio ER request"),
        )
        .await
        .expect("Studio ER response");
    assert_eq!(er_diagram.status(), StatusCode::OK);
    let er_diagram = response_text(er_diagram).await;
    assert!(
        er_diagram.contains(table),
        "ER diagram omitted matrix table: {er_diagram}"
    );
    assert!(er_diagram.contains(&parent_table));
    assert!(er_diagram.contains("parent_id_to_id"));

    feature_flags::exercise_feature_flags(&app, pool, driver).await;

    let initial_table = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/studio/tables/{table}"))
                .body(Body::empty())
                .expect("valid Studio table request"),
        )
        .await
        .expect("Studio table response");
    assert_eq!(initial_table.status(), StatusCode::OK);
    let initial_table = response_text(initial_table).await;
    assert!(initial_table.contains("before"));
    assert!(initial_table.contains("PK"));
    assert!(initial_table.contains("Choose column"));

    let searched_table = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/studio/tables/{table}?search=before&page=1000001"))
                .header("hx-request", "true")
                .body(Body::empty())
                .expect("valid Studio HTMX search request"),
        )
        .await
        .expect("Studio HTMX table response");
    assert_eq!(searched_table.status(), StatusCode::OK);
    let searched_table = response_text(searched_table).await;
    assert!(searched_table.contains("No records found"));
    assert!(!searched_table.contains("<!DOCTYPE html>"));

    let missing_table = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/studio/tables/not_present")
                .header("hx-request", "true")
                .body(Body::empty())
                .expect("valid missing-table request"),
        )
        .await
        .expect("Studio missing-table response");
    assert_eq!(missing_table.status(), StatusCode::NOT_FOUND);

    let unsafe_table = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/studio/tables/unsafe%3Bdrop")
                .body(Body::empty())
                .expect("valid unsafe-table request"),
        )
        .await
        .expect("Studio unsafe-table response");
    assert_eq!(unsafe_table.status(), StatusCode::NOT_FOUND);

    let update = app
        .clone()
        .oneshot(form_request("update", "column=name&value=after&pk_id=7"))
        .await
        .expect("Studio update response");
    assert!(update.status().is_redirection());

    let mut select = QueryBuilder::<rullst_orm::RullstDatabase>::new("SELECT name FROM ");
    select
        .push(&quoted_table)
        .push(" WHERE id = ")
        .push_bind(7_i64);
    let name: String = fixture(&mut select, driver)
        .fetch_one(pool)
        .await
        .and_then(|row| _sqlx::Row::try_get(&row, 0))
        .expect("read Studio-updated row");
    assert_eq!(name, "after");

    let boolean_update = app
        .clone()
        .oneshot(form_request("update", "column=active&value=false&pk_id=7"))
        .await
        .expect("Studio boolean update response");
    assert!(boolean_update.status().is_redirection());

    let null_update = app
        .clone()
        .oneshot(form_request(
            "update",
            "column=score&value=&set_null=true&pk_id=7",
        ))
        .await
        .expect("Studio nullable update response");
    assert!(null_update.status().is_redirection());

    let mut typed_select = QueryBuilder::<rullst_orm::RullstDatabase>::new(
        "SELECT CASE WHEN active THEN 'true' ELSE 'false' END AS active_text, score FROM ",
    );
    typed_select
        .push(&quoted_table)
        .push(" WHERE id = ")
        .push_bind(7_i64);
    let typed_row = fixture(&mut typed_select, driver)
        .fetch_one(pool)
        .await
        .expect("read Studio typed updates");
    let active: String =
        _sqlx::Row::try_get(&typed_row, 0).expect("Studio-updated boolean projection");
    let score: Option<f64> =
        _sqlx::Row::try_get(&typed_row, 1).expect("Studio-updated nullable value");
    assert_eq!(active, "false");
    assert_eq!(score, None);

    let invalid = app
        .clone()
        .oneshot(form_request("update", "column=id&value=8&pk_id=7"))
        .await
        .expect("Studio primary-key rejection");
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);

    for invalid_body in [
        "column=name&value=blocked&set_null=true&pk_id=7",
        "column=name&value=blocked&set_null=maybe&pk_id=7",
        "column=unknown&value=blocked&pk_id=7",
        "column=name&pk_id=7",
        "column=name&value=blocked&pk_id=7&unexpected=true",
        "column=name&value=blocked&pk_id=not-an-integer",
    ] {
        let invalid = app
            .clone()
            .oneshot(form_request("update", invalid_body))
            .await
            .expect("Studio invalid typed update response");
        assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    let missing_row = app
        .clone()
        .oneshot(form_request(
            "update",
            "column=name&value=blocked&pk_id=9999",
        ))
        .await
        .expect("Studio missing-row update response");
    assert_eq!(missing_row.status(), StatusCode::NOT_FOUND);

    let wrong_confirmation = app
        .clone()
        .oneshot(form_request("delete", "confirm=DELETE+wrong_table&pk_id=7"))
        .await
        .expect("Studio delete-confirmation rejection");
    assert_eq!(
        wrong_confirmation.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let missing_delete = app
        .clone()
        .oneshot(form_request(
            "delete",
            &format!("confirm=DELETE+{table}&pk_id=9999"),
        ))
        .await
        .expect("Studio missing-row delete response");
    assert_eq!(missing_delete.status(), StatusCode::NOT_FOUND);

    let raw_router_denial = rullst_studio::data_browser::router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/studio/tables/{table}/rows/delete"))
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("confirm=DELETE+{table}&pk_id=7")))
                .expect("valid raw-router denial request"),
        )
        .await
        .expect("Studio raw-router denial response");
    assert_eq!(raw_router_denial.status(), StatusCode::FORBIDDEN);

    let confirmation = format!("confirm=DELETE+{table}&pk_id=7");
    let deletion = Request::builder()
        .method("POST")
        .uri(format!("/studio/tables/{table}/rows/delete"))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(confirmation))
        .expect("valid Studio deletion request");
    let response = app
        .clone()
        .oneshot(deletion)
        .await
        .expect("Studio delete response");
    assert!(response.status().is_redirection());

    let mut count = QueryBuilder::<rullst_orm::RullstDatabase>::new("SELECT COUNT(*) FROM ");
    count
        .push(&quoted_table)
        .push(" WHERE id = ")
        .push_bind(7_i64);
    let remaining: i64 = fixture(&mut count, driver)
        .fetch_one(pool)
        .await
        .and_then(|row| _sqlx::Row::try_get(&row, 0))
        .expect("read Studio deletion result");
    assert_eq!(remaining, 0);

    let empty_table = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/studio/tables/{table}?search=after"))
                .body(Body::empty())
                .expect("valid empty-table request"),
        )
        .await
        .expect("Studio empty-table response");
    assert_eq!(empty_table.status(), StatusCode::OK);
    assert!(
        response_text(empty_table)
            .await
            .contains("No records found")
    );

    incomplete_key::exercise_incomplete_primary_key(&app, pool, driver, table).await;
    bounded_view::exercise_bounded_table_view(&app, pool, driver, table).await;
}

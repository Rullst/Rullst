use axum::{
    body::Body,
    extract::{ConnectInfo, Request as AxumRequest},
    http::{Request, StatusCode, header},
    middleware::Next,
};
use rullst_studio::{LocalStudioAccess, Studio};
use std::net::SocketAddr;
use tower::ServiceExt;

#[cfg(any(feature = "strict-postgres", feature = "strict-mysql"))]
pub fn handle_container_start_error(provider: &str, error: impl std::fmt::Display) {
    if std::env::var("RULLST_REQUIRE_TESTCONTAINERS").as_deref() == Ok("true") {
        panic!("{provider} testcontainer is required but failed to start: {error}");
    }
    eprintln!("skipping {provider} Studio mutation matrix: {error}");
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

pub async fn exercise_mutations(database_url: &str, driver: &str, table: &str) {
    assert!(
        !table.is_empty()
            && table.len() <= 64
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
    let mut create =
        rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new("CREATE TABLE ");
    create.push(&quoted_table).push(
        " (id BIGINT PRIMARY KEY, name VARCHAR(255) NOT NULL, active BOOLEAN NOT NULL, \
         score DOUBLE PRECISION NULL)",
    );
    create
        .build()
        .execute(pool)
        .await
        .expect("create Studio matrix table");
    let mut insert =
        rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new("INSERT INTO ");
    insert
        .push(&quoted_table)
        .push(" (id, name, active, score) VALUES (")
        .push_bind(7_i64)
        .push(", ")
        .push_bind("before")
        .push(", ")
        .push_bind(true)
        .push(", ")
        .push_bind(1.5_f64)
        .push(")");
    insert
        .build()
        .execute(pool)
        .await
        .expect("insert Studio matrix row");

    let app = Studio::new()
        .into_router(LocalStudioAccess::loopback_only())
        .expect("debug Studio router")
        .layer(axum::middleware::from_fn(inject_loopback));
    let form_request = |suffix: &str, body: &'static str| {
        Request::builder()
            .method("POST")
            .uri(format!("/studio/tables/{table}/rows/{suffix}"))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .expect("valid Studio matrix request")
    };

    let update = app
        .clone()
        .oneshot(form_request("update", "column=name&value=after&pk_id=7"))
        .await
        .expect("Studio update response");
    assert!(update.status().is_redirection());

    let mut select =
        rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new("SELECT name FROM ");
    select
        .push(&quoted_table)
        .push(" WHERE id = ")
        .push_bind(7_i64);
    let name: String = select
        .build_query_scalar()
        .fetch_one(pool)
        .await
        .expect("read Studio-updated row");
    assert_eq!(name, "after");

    let invalid = app
        .clone()
        .oneshot(form_request("update", "column=id&value=8&pk_id=7"))
        .await
        .expect("Studio primary-key rejection");
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let wrong_confirmation = app
        .clone()
        .oneshot(form_request("delete", "confirm=DELETE+wrong_table&pk_id=7"))
        .await
        .expect("Studio delete-confirmation rejection");
    assert_eq!(
        wrong_confirmation.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );

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
    let response = app.oneshot(deletion).await.expect("Studio delete response");
    assert!(response.status().is_redirection());

    let mut count =
        rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new("SELECT COUNT(*) FROM ");
    count
        .push(&quoted_table)
        .push(" WHERE id = ")
        .push_bind(7_i64);
    let remaining: i64 = count
        .build_query_scalar()
        .fetch_one(pool)
        .await
        .expect("read Studio deletion result");
    assert_eq!(remaining, 0);
}

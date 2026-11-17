//! Row-action form bounds shared by the Studio mutation matrix: a key of
//! exactly 16 KiB stays addressable, and the delete form is offered exactly
//! up to the 64 KiB mutation body limit. MySQL and MariaDB cannot index keys
//! this long, so only SQLite and PostgreSQL run it.

use super::{fixture, response_text};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

pub(super) async fn exercise_form_bounds(
    app: &axum::Router,
    pool: &rullst_orm::RullstPool,
    driver: &str,
    table: &str,
) {
    let forms_table = format!("{table}_forms");
    let quoted_table = format!("\"{forms_table}\"");
    let mut create =
        rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new("CREATE TABLE ");
    create
        .push(&quoted_table)
        .push(" (a TEXT NOT NULL, b TEXT NOT NULL, note TEXT NULL, PRIMARY KEY (a, b))");
    fixture(&mut create, driver)
        .execute(pool)
        .await
        .expect("create Studio form-bounds table");

    // The delete form sends `pk_a=…&pk_b=…&` (6 + 6 bytes plus the encoded
    // values) and `&confirm=DELETE+<table>` (9 + 7 + table bytes). A `%` is
    // sent as `%25`, three bytes.
    let budget = 65_536 - 12 - (16 + forms_table.len());
    let wide = "%".repeat(16_384);
    let fitting = "x".repeat(budget - 49_152);
    let oversized = "x".repeat(budget - 49_152 + 1);
    let rows = [
        // Exactly 64 KiB: the delete form is offered, the edit form is not.
        (wide.clone(), fitting.clone()),
        // One byte more: the row is read-only.
        (wide, oversized.clone()),
        // A key of exactly 16 KiB is addressable.
        ("y".repeat(16_384), "z".to_string()),
        // One byte more is not.
        ("y".repeat(16_385), "z".to_string()),
        // A short key offers the edit form for the non-key column only.
        ("p".to_string(), "q".to_string()),
    ];
    for (a, b) in rows {
        let mut insert =
            rullst_orm::_sqlx::QueryBuilder::<rullst_orm::RullstDatabase>::new("INSERT INTO ");
        insert
            .push(&quoted_table)
            .push(" (a, b) VALUES (")
            .push_bind(a)
            .push(", ")
            .push_bind(b)
            .push(")");
        fixture(&mut insert, driver)
            .execute(pool)
            .await
            .expect("insert Studio form-bounds row");
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/studio/tables/{forms_table}"))
                .header("hx-request", "true")
                .body(Body::empty())
                .expect("valid form-bounds request"),
        )
        .await
        .expect("Studio form-bounds response");
    assert_eq!(response.status(), StatusCode::OK);
    let html = response_text(response).await;
    assert_eq!(
        html.matches("Read-only: key too large for the 64 KiB form limit")
            .count(),
        1
    );
    assert_eq!(html.matches("Read-only: key longer than 16 KiB").count(), 1);
    assert_eq!(
        html.matches("Edit unavailable: key too large for the 64 KiB form limit")
            .count(),
        2
    );
    assert_eq!(html.matches(">Delete row</button>").count(), 3);
    // The row of exactly 64 KiB is the one that keeps its forms.
    assert!(html.contains(&format!("name=\"pk_b\" value=\"{fitting}\"")));
    assert!(!html.contains(&format!("name=\"pk_b\" value=\"{oversized}\"")));
    assert_eq!(html.matches(">Apply</button>").count(), 1);
    assert!(html.contains("<option value=\"note\">note (nullable)</option>"));
    assert!(!html.contains("<option value=\"a\">"));
    assert!(!html.contains("<option value=\"b\">"));
}

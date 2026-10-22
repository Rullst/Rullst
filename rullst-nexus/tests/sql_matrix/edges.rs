//! Cases only a real server produces: floating-point primary keys, a batch
//! on a table that is gone, and an `INSERT` a PostgreSQL rule discards.

use super::{execute, send};
use crate::support::authenticated_test_router;
use axum::http::StatusCode;
use rullst_nexus::{FieldKind, FieldMeta, Nexus, NexusModel};
use rullst_orm::RullstPool;

struct Measurement;

impl NexusModel for Measurement {
    fn nexus_table() -> &'static str {
        "nexus_matrix_measurements"
    }
    fn nexus_label() -> &'static str {
        "Measurements"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("label", "Label", FieldKind::Text),
        ]
    }
}

struct Discarded;

impl NexusModel for Discarded {
    fn nexus_table() -> &'static str {
        "nexus_matrix_discarded"
    }
    fn nexus_label() -> &'static str {
        "Discarded"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("label", "Label", FieldKind::Text),
        ]
    }
}

pub async fn edge_cases(pool: &RullstPool, driver: &str) {
    let float = if driver == "postgres" {
        "DOUBLE PRECISION"
    } else {
        "DOUBLE"
    };
    execute(
        pool,
        &format!(
            "CREATE TABLE nexus_matrix_measurements (id {float} PRIMARY KEY, \
             label VARCHAR(64) NOT NULL)"
        ),
    )
    .await;
    execute(
        pool,
        "INSERT INTO nexus_matrix_measurements (id, label) VALUES \
         (2, 'whole'), (2.5, 'fraction'), (9007199254740992, 'beyond')",
    )
    .await;
    let app = authenticated_test_router(
        Nexus::new()
            .register::<Measurement>()
            .register::<Discarded>(),
    );

    // Only an exactly representable whole number addresses a record.
    let (status, list) = send(&app, "GET", "/table/nexus_matrix_measurements", "").await;
    assert_eq!(status, StatusCode::OK, "{driver}: {list}");
    let start = list
        .find("data-nexus-row-id=\"2\"")
        .unwrap_or_else(|| panic!("{driver}: no row addressed as 2: {list}"));
    let end = list[start..].find("</tr>").expect("row end") + start;
    assert!(
        list[start..end].contains("<td class=\"nexus-td\">whole</td>"),
        "{driver}: {list}"
    );
    assert_eq!(list.matches("No usable key").count(), 2, "{driver}: {list}");

    execute(pool, "DROP TABLE nexus_matrix_measurements").await;
    let (status, body) = send(
        &app,
        "POST",
        "/table/nexus_matrix_measurements/batch",
        "action=delete&selected_ids=2",
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{driver}");
    assert_eq!(
        body,
        "The requested batch operation could not be completed."
    );

    if driver == "postgres" {
        execute(
            pool,
            "CREATE TABLE nexus_matrix_discarded (id SERIAL PRIMARY KEY, \
             label VARCHAR(64) NOT NULL)",
        )
        .await;
        execute(
            pool,
            "CREATE RULE nexus_matrix_discard AS ON INSERT TO nexus_matrix_discarded \
             DO INSTEAD NOTHING",
        )
        .await;
        // The statement succeeds but writes no row: not a created record.
        let (status, body) =
            send(&app, "POST", "/table/nexus_matrix_discarded", "label=kept").await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
        assert_eq!(
            body,
            "The requested database operation could not be completed."
        );
    }
}

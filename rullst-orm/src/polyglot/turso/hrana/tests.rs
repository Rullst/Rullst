use serde_json::json;

use super::*;

fn statement(sql: &str, parameters: Vec<TursoValue>) -> TursoStatement {
    TursoStatement::new(sql, parameters).expect("valid test statement")
}

#[test]
fn maps_supported_base_urls_to_the_v3_pipeline() {
    assert_eq!(
        pipeline_url("libsql://database.turso.io")
            .expect("valid libsql endpoint")
            .as_str(),
        "https://database.turso.io/v3/pipeline"
    );
    assert_eq!(
        pipeline_url("http://127.0.0.1:8080")
            .expect("valid loopback endpoint")
            .as_str(),
        "http://127.0.0.1:8080/v3/pipeline"
    );
}

#[test]
fn encodes_lossless_hrana_scalars() {
    let wire = WireStatement::new(
        statement(
            "SELECT ?1, ?2",
            vec![
                TursoValue::Integer(i64::MAX),
                TursoValue::Blob(vec![0, 1, 255]),
            ],
        ),
        true,
    );
    let encoded = serde_json::to_value(wire).expect("wire statement should serialize");
    assert_eq!(
        encoded,
        json!({
            "sql": "SELECT ?1, ?2",
            "args": [
                {"type": "integer", "value": i64::MAX.to_string()},
                {"type": "blob", "base64": "AAH/"}
            ],
            "want_rows": true
        })
    );
}

#[test]
fn builds_a_conditional_atomic_batch() {
    let batch = transactional_batch(vec![
        statement(
            "INSERT INTO events VALUES (?1)",
            vec![TursoValue::Integer(1)],
        ),
        statement("UPDATE events SET id = ?1", vec![TursoValue::Integer(2)]),
    ])
    .expect("bounded transaction should serialize");
    let encoded = serde_json::to_value(batch).expect("batch should serialize");
    let steps = encoded["steps"].as_array().expect("steps array");
    assert_eq!(steps.len(), 5);
    assert_eq!(steps[0]["stmt"]["sql"], "BEGIN TRANSACTION");
    assert_eq!(steps[1]["condition"], json!({"type": "ok", "step": 0}));
    assert_eq!(steps[3]["stmt"]["sql"], "COMMIT");
    assert_eq!(
        steps[4]["condition"],
        json!({"type": "not", "cond": {"type": "ok", "step": 3}})
    );
    assert_eq!(steps[4]["stmt"]["sql"], "ROLLBACK");
}

#[test]
fn decodes_rows_and_honors_the_materialization_limit() {
    let result = StatementResult {
        cols: vec![
            Column {
                name: Some("id".to_owned()),
            },
            Column {
                name: Some("payload".to_owned()),
            },
        ],
        rows: vec![
            vec![
                WireValue::Integer {
                    value: "7".to_owned(),
                },
                WireValue::Blob {
                    base64: "AAH/".to_owned(),
                },
            ],
            vec![
                WireValue::Integer {
                    value: "8".to_owned(),
                },
                WireValue::Null,
            ],
        ],
        affected_row_count: 0,
    };
    let rows = rows_from_result(result, TursoQueryLimit::new(1).expect("valid limit"))
        .expect("valid Hrana rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("id"), Some(&TursoValue::Integer(7)));
    assert_eq!(
        rows[0].get("payload"),
        Some(&TursoValue::Blob(vec![0, 1, 255]))
    );
}

#[test]
fn rejects_malformed_hrana_rows() {
    let result = StatementResult {
        cols: vec![Column {
            name: Some("id".to_owned()),
        }],
        rows: vec![vec![WireValue::Integer {
            value: "not-an-integer".to_owned(),
        }]],
        affected_row_count: 0,
    };
    assert!(rows_from_result(result, TursoQueryLimit::new(1).expect("valid limit")).is_err());
}

#[test]
fn decodes_unpadded_and_padded_blob_cells() {
    // libSQL server writes blobs as unpadded standard base64; lengths that
    // are not a multiple of three need padding in canonical form.
    for (encoded, expected) in [
        ("AA", vec![0]),
        ("AA==", vec![0]),
        ("AAE", vec![0, 1]),
        ("AAE=", vec![0, 1]),
        ("AAECAwQFBgcICQoLDA0ODw", (0..16).collect::<Vec<u8>>()),
        ("AAECAwQFBgcICQoLDA0ODw==", (0..16).collect::<Vec<u8>>()),
        ("AAH/", vec![0, 1, 255]),
        ("", Vec::new()),
    ] {
        let decoded = TursoValue::try_from(WireValue::Blob {
            base64: encoded.to_owned(),
        })
        .unwrap_or_else(|error| panic!("{encoded:?} should decode: {error}"));
        assert_eq!(decoded, TursoValue::Blob(expected), "{encoded:?}");
    }
    for malformed in ["A", "AA=A", "AA!=", "AB"] {
        assert!(
            TursoValue::try_from(WireValue::Blob {
                base64: malformed.to_owned(),
            })
            .is_err(),
            "{malformed:?} must be rejected"
        );
    }
}

fn coded(code: Option<&str>, message: &str) -> PolyglotError {
    server_error(ServerError {
        message: message.to_owned(),
        code: code.map(str::to_owned),
    })
}

#[test]
fn unique_and_primary_key_violations_are_conflicts() {
    // libSQL server v0.24 sends the primary code and SQLite's fixed text.
    for message in [
        "SQLite error: UNIQUE constraint failed: users.email",
        "SQLite error: UNIQUE constraint failed: users.id",
    ] {
        assert!(
            matches!(
                coded(Some("SQLITE_CONSTRAINT"), message),
                PolyglotError::Conflict
            ),
            "{message}"
        );
    }
    for code in ["SQLITE_CONSTRAINT_UNIQUE", "SQLITE_CONSTRAINT_PRIMARYKEY"] {
        assert!(
            matches!(coded(Some(code), "constraint"), PolyglotError::Conflict),
            "{code}"
        );
    }
}

#[test]
fn other_constraint_and_server_errors_stay_driver_errors() {
    for (code, message, expected) in [
        (
            Some("SQLITE_CONSTRAINT"),
            "SQLite error: NOT NULL constraint failed: users.name",
            "Hrana server error SQLITE_CONSTRAINT: SQLite error: NOT NULL constraint failed: users.name",
        ),
        (
            Some("SQLITE_CONSTRAINT_NOTNULL"),
            "UNIQUE constraint failed: users.id",
            "Hrana server error SQLITE_CONSTRAINT_NOTNULL: UNIQUE constraint failed: users.id",
        ),
        // The text alone is not trusted without the constraint code.
        (
            Some("SQLITE_ERROR"),
            "UNIQUE constraint failed: users.id",
            "Hrana server error SQLITE_ERROR: UNIQUE constraint failed: users.id",
        ),
        (
            None,
            "UNIQUE constraint failed: users.id",
            "Hrana server error: UNIQUE constraint failed: users.id",
        ),
    ] {
        match coded(code, message) {
            PolyglotError::Driver { backend, message } => {
                assert_eq!(backend, "Turso");
                assert_eq!(message, expected);
            }
            other => panic!("{code:?} {message}: {other:?}"),
        }
    }
}

use std::sync::Arc;

use duckdb::types::{Decimal, TimeUnit, Value, ValueRef};

use super::*;

#[tokio::test]
async fn executes_bound_parameters_and_bounded_queries() {
    let store = DuckDbStore::in_memory().await.unwrap();
    store
        .execute(
            "CREATE TABLE events (sequence BIGINT, label VARCHAR)",
            vec![],
        )
        .await
        .unwrap();
    for sequence in 1..=3 {
        store
            .execute(
                "INSERT INTO events VALUES (?, ?)",
                vec![
                    AnalyticsValue::Signed(sequence),
                    AnalyticsValue::Text(format!("event-{sequence}")),
                ],
            )
            .await
            .unwrap();
    }

    let rows = store
        .query(
            "SELECT sequence, label FROM events WHERE sequence >= ? ORDER BY sequence",
            vec![AnalyticsValue::Signed(2)],
            QueryLimit::new(1).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("sequence"), Some(&AnalyticsValue::Signed(2)));
    assert_eq!(
        rows[0].get("label"),
        Some(&AnalyticsValue::Text("event-2".to_owned()))
    );
    assert_eq!(rows.into_iter().next().unwrap().into_columns().len(), 2);
}

#[test]
fn validates_limits_capabilities_and_payload_bounds() {
    assert_eq!(QueryLimit::new(10_000).unwrap().get(), 10_000);
    assert!(QueryLimit::new(0).is_err());
    assert!(QueryLimit::new(10_001).is_err());
    assert!(validate_analytics_request("", &[]).is_err());
    assert!(validate_analytics_request(&"x".repeat(MAX_SQL_BYTES + 1), &[]).is_err());
    assert!(validate_analytics_request("SELECT ?", &vec![AnalyticsValue::Null; 1_025]).is_err());
    assert!(
        validate_analytics_request(
            "SELECT ?",
            &[AnalyticsValue::Bytes(vec![0; MAX_PARAMETER_BYTES + 1])],
        )
        .is_err()
    );

    let capabilities = DuckDbStore::capabilities();
    assert_eq!(capabilities.backend(), Backend::DuckDb);
    assert!(capabilities.supports(Capability::Analytics));
    assert!(!capabilities.supports(Capability::Documents));
}

#[test]
fn converts_every_portable_parameter_variant() {
    let decimal = Decimal::new(9, 2, 12_345).unwrap();
    let cases = [
        (AnalyticsValue::Null, Value::Null),
        (AnalyticsValue::Boolean(true), Value::Boolean(true)),
        (AnalyticsValue::Signed(-42), Value::HugeInt(-42)),
        (AnalyticsValue::Unsigned(42), Value::UHugeInt(42)),
        (AnalyticsValue::Float(1.25), Value::Double(1.25)),
        (
            AnalyticsValue::Text("analytics".to_owned()),
            Value::Text("analytics".to_owned()),
        ),
        (
            AnalyticsValue::Bytes(vec![1, 2, 3]),
            Value::Blob(vec![1, 2, 3]),
        ),
        (
            AnalyticsValue::Decimal {
                precision: 9,
                scale: 2,
                scaled: 12_345,
            },
            Value::Decimal(decimal),
        ),
        (
            AnalyticsValue::Timestamp {
                unit: AnalyticsTimeUnit::Microsecond,
                value: 123,
            },
            Value::Timestamp(TimeUnit::Microsecond, 123),
        ),
        (AnalyticsValue::DateDays(20_000), Value::Date32(20_000)),
        (
            AnalyticsValue::Time {
                unit: AnalyticsTimeUnit::Nanosecond,
                value: 456,
            },
            Value::Time64(TimeUnit::Nanosecond, 456),
        ),
        (
            AnalyticsValue::Interval {
                months: 1,
                days: 2,
                nanos: 3,
            },
            Value::Interval {
                months: 1,
                days: 2,
                nanos: 3,
            },
        ),
        (
            AnalyticsValue::Geometry(vec![1, 2, 3]),
            Value::Geometry(vec![1, 2, 3]),
        ),
    ];

    for (portable, native) in cases {
        assert_eq!(to_duckdb_value(portable).unwrap(), native);
    }
    assert!(matches!(
        to_duckdb_value(AnalyticsValue::Decimal {
            precision: 0,
            scale: 0,
            scaled: 0,
        }),
        Err(PolyglotError::UnsupportedValue {
            backend: "DuckDB",
            ..
        })
    ));
}

#[test]
fn converts_every_supported_native_scalar() {
    let decimal = Decimal::new(7, 3, 12_345).unwrap();
    let cases = [
        (Value::Null, AnalyticsValue::Null),
        (Value::Boolean(true), AnalyticsValue::Boolean(true)),
        (Value::TinyInt(-1), AnalyticsValue::Signed(-1)),
        (Value::SmallInt(-2), AnalyticsValue::Signed(-2)),
        (Value::Int(-3), AnalyticsValue::Signed(-3)),
        (Value::BigInt(-4), AnalyticsValue::Signed(-4)),
        (Value::HugeInt(-5), AnalyticsValue::Signed(-5)),
        (Value::UTinyInt(1), AnalyticsValue::Unsigned(1)),
        (Value::USmallInt(2), AnalyticsValue::Unsigned(2)),
        (Value::UInt(3), AnalyticsValue::Unsigned(3)),
        (Value::UBigInt(4), AnalyticsValue::Unsigned(4)),
        (Value::UHugeInt(5), AnalyticsValue::Unsigned(5)),
        (Value::Float(1.5), AnalyticsValue::Float(1.5)),
        (Value::Double(2.5), AnalyticsValue::Float(2.5)),
        (
            Value::Decimal(decimal),
            AnalyticsValue::Decimal {
                precision: 7,
                scale: 3,
                scaled: 12_345,
            },
        ),
        (
            Value::Timestamp(TimeUnit::Second, 10),
            AnalyticsValue::Timestamp {
                unit: AnalyticsTimeUnit::Second,
                value: 10,
            },
        ),
        (
            Value::Text("valid UTF-8".to_owned()),
            AnalyticsValue::Text("valid UTF-8".to_owned()),
        ),
        (Value::Blob(vec![4, 5]), AnalyticsValue::Bytes(vec![4, 5])),
        (
            Value::Geometry(vec![6, 7]),
            AnalyticsValue::Geometry(vec![6, 7]),
        ),
        (Value::Date32(20_001), AnalyticsValue::DateDays(20_001)),
        (
            Value::Time64(TimeUnit::Millisecond, 11),
            AnalyticsValue::Time {
                unit: AnalyticsTimeUnit::Millisecond,
                value: 11,
            },
        ),
        (
            Value::Interval {
                months: 8,
                days: 9,
                nanos: 10,
            },
            AnalyticsValue::Interval {
                months: 8,
                days: 9,
                nanos: 10,
            },
        ),
    ];

    for (native, portable) in &cases {
        assert_eq!(
            from_duckdb_value(ValueRef::from(native)).unwrap(),
            *portable
        );
    }
    assert!(matches!(
        from_duckdb_value(ValueRef::Text(&[0xff])),
        Err(PolyglotError::Serialization(_))
    ));
}

#[test]
fn maps_all_temporal_units_in_both_directions() {
    let cases = [
        (AnalyticsTimeUnit::Second, TimeUnit::Second),
        (AnalyticsTimeUnit::Millisecond, TimeUnit::Millisecond),
        (AnalyticsTimeUnit::Microsecond, TimeUnit::Microsecond),
        (AnalyticsTimeUnit::Nanosecond, TimeUnit::Nanosecond),
    ];
    for (portable, native) in cases {
        assert_eq!(to_duckdb_time_unit(portable), native);
        assert_eq!(from_duckdb_time_unit(native), portable);
    }
}

#[tokio::test]
async fn reports_query_shape_and_driver_failures_without_partial_results() {
    let store = DuckDbStore::in_memory().await.unwrap();
    let duplicate = store
        .query(
            "SELECT 1 AS duplicate, 2 AS duplicate",
            vec![],
            QueryLimit::new(1).unwrap(),
        )
        .await;
    assert!(matches!(
        duplicate,
        Err(PolyglotError::Driver {
            backend: "DuckDB",
            ..
        })
    ));

    let nested = store
        .query(
            "SELECT [1, 2] AS nested",
            vec![],
            QueryLimit::new(1).unwrap(),
        )
        .await;
    assert!(matches!(
        nested,
        Err(PolyglotError::UnsupportedValue {
            backend: "DuckDB",
            ..
        })
    ));
    assert!(matches!(
        store.execute("not valid SQL", vec![]).await,
        Err(PolyglotError::Driver { .. })
    ));
    assert!(matches!(
        store
            .query("not valid SQL", vec![], QueryLimit::new(1).unwrap())
            .await,
        Err(PolyglotError::Driver { .. })
    ));
}

#[tokio::test]
async fn opens_file_backed_databases_and_reports_invalid_paths() {
    let path = std::env::temp_dir().join(format!("rullst-duckdb-{}.db", uuid::Uuid::new_v4()));
    let store = DuckDbStore::open(path.clone()).await.unwrap();
    store
        .execute("CREATE TABLE durable (id INT)", vec![])
        .await
        .unwrap();
    drop(store);

    let reopened = DuckDbStore::open(path.clone()).await.unwrap();
    let rows = reopened
        .query(
            "SELECT COUNT(*) AS count FROM durable",
            vec![],
            QueryLimit::new(1).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rows[0].get("count"), Some(&AnalyticsValue::Signed(0)));
    drop(reopened);
    std::fs::remove_file(&path).unwrap();

    let missing_parent = std::env::temp_dir()
        .join(format!("rullst-missing-{}", uuid::Uuid::new_v4()))
        .join("database.duckdb");
    assert!(matches!(
        DuckDbStore::open(missing_parent).await,
        Err(PolyglotError::Driver {
            backend: "DuckDB",
            ..
        })
    ));
}

#[tokio::test]
async fn poisoned_connections_and_failed_workers_return_typed_errors() {
    let store = DuckDbStore::in_memory().await.unwrap();
    let connection = Arc::clone(&store.connection);
    let _ = std::thread::spawn(move || {
        let _guard = connection.lock().unwrap();
        panic!("poison the test-only connection lock");
    })
    .join();

    assert!(matches!(
        store.execute("SELECT 1", vec![]).await,
        Err(PolyglotError::Worker {
            backend: "DuckDB",
            ..
        })
    ));
    assert!(matches!(
        store
            .query("SELECT 1", vec![], QueryLimit::new(1).unwrap())
            .await,
        Err(PolyglotError::Worker {
            backend: "DuckDB",
            ..
        })
    ));

    let join_error = tokio::spawn(async { panic!("test-only worker failure") })
        .await
        .unwrap_err();
    assert!(matches!(
        worker_error(join_error),
        PolyglotError::Worker {
            backend: "DuckDB",
            ..
        }
    ));
}

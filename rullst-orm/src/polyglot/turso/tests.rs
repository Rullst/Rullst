use super::*;

#[test]
fn capabilities_include_the_bounded_typed_model_profile() {
    let capabilities = TursoStore::capabilities();
    assert!(capabilities.supports(Capability::EdgeSql));
    assert!(capabilities.supports(Capability::RelationalModels));
    assert!(!capabilities.supports(Capability::Documents));
}

fn statement(sql: &str, parameters: Vec<TursoValue>) -> TursoStatement {
    TursoStatement::new(sql, parameters).unwrap()
}

#[tokio::test]
async fn offline_fallback_executes_real_parameterized_sql() {
    let store = TursoStore::connect(TursoConfig::new("mock_local", ""))
        .await
        .unwrap();
    assert!(store.is_offline());
    store
        .execute(statement(
            "CREATE TABLE events (sequence INTEGER, label TEXT)",
            vec![],
        ))
        .await
        .unwrap();
    store
        .transaction(vec![
            statement(
                "INSERT INTO events VALUES (?1, ?2)",
                vec![TursoValue::Integer(1), TursoValue::Text("one".into())],
            ),
            statement(
                "INSERT INTO events VALUES (?1, ?2)",
                vec![TursoValue::Integer(2), TursoValue::Text("two".into())],
            ),
        ])
        .await
        .unwrap();
    let rows = store
        .query(
            statement(
                "SELECT sequence, label FROM events WHERE sequence >= ?1 ORDER BY sequence",
                vec![TursoValue::Integer(1)],
            ),
            TursoQueryLimit::new(1).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("sequence"), Some(&TursoValue::Integer(1)));
    assert_eq!(rows[0].get("label"), Some(&TursoValue::Text("one".into())));
}

#[test]
fn validates_transport_tokens_statements_and_bounds() {
    assert!(
        TursoConfig::new("https://example.turso.io", "")
            .validate()
            .is_err()
    );
    assert!(
        TursoConfig::new("http://example.turso.io", "token")
            .validate()
            .is_err()
    );
    assert!(
        TursoConfig::new("ftp://127.0.0.1:8080", "")
            .allow_insecure_loopback()
            .validate()
            .is_err()
    );
    assert!(
        TursoConfig::new("http://127.0.0.1:8080", "")
            .allow_insecure_loopback()
            .validate()
            .is_ok()
    );
    assert!(TursoStatement::new("", vec![]).is_err());
    assert!(TursoStatement::new("SELECT 1", vec![TursoValue::Null; 1_025]).is_err());
    assert!(
        TursoStatement::new(
            "SELECT ?1",
            vec![TursoValue::Blob(vec![0; MAX_PARAMETER_BYTES + 1])],
        )
        .is_err()
    );
    assert!(TursoQueryLimit::new(0).is_err());
    assert!(TursoQueryLimit::new(10_001).is_err());
    assert!(!format!("{:?}", TursoConfig::new("mock_local", "secret")).contains("secret"));
}

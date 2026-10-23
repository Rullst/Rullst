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
    let columns = rows[0].clone().into_columns();
    assert_eq!(
        columns.into_iter().collect::<Vec<_>>(),
        [
            ("label".to_owned(), TursoValue::Text("one".into())),
            ("sequence".to_owned(), TursoValue::Integer(1)),
        ]
    );
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
    assert!(
        TursoConfig::new("http://[::1]:8080", "")
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

#[test]
fn statements_reach_exactly_their_sql_and_parameter_bounds() {
    let mib = 1024 * 1024;
    assert!(TursoStatement::new("x".repeat(mib), vec![]).is_ok());
    assert!(TursoStatement::new("x".repeat(mib + 1), vec![]).is_err());
    assert!(TursoStatement::new("SELECT ?1", vec![TursoValue::Blob(vec![0; 8 * mib])]).is_ok());
    // SQL plus parameters: text and blobs count their bytes, numbers eight.
    let payload = statement(
        "SELECT ?1, ?2, ?3, ?4, ?5",
        vec![
            TursoValue::Null,
            TursoValue::Integer(1),
            TursoValue::Real(1.5),
            TursoValue::Text("abc".into()),
            TursoValue::Blob(vec![0; 5]),
        ],
    );
    assert_eq!(payload.payload_bytes(), Some(25 + 8 + 8 + 3 + 5));
    assert_eq!(TursoQueryLimit::new(10_000).unwrap().get(), 10_000);
    assert_eq!(TursoQueryLimit::new(7).unwrap().get(), 7);
}

#[test]
fn remote_configuration_is_bare_secure_and_authenticated() {
    let invalid = |url: &str| TursoConfig::new(url, "token").validate().is_err();
    for url in [
        "libsql://user@db.turso.io",
        "libsql://:pw@db.turso.io",
        "libsql://db.turso.io?x=1",
        "libsql://db.turso.io#x",
        "libsql:///no-host",
    ] {
        assert!(invalid(url), "{url}");
    }
    assert!(
        TursoConfig::new("libsql://db.turso.io", "token")
            .validate()
            .is_ok()
    );
    assert!(
        TursoConfig::new("https://db.turso.io", "token")
            .validate()
            .is_ok()
    );
    assert!(
        TursoConfig::new("https://db.turso.io", " ")
            .validate()
            .is_err()
    );
    assert!(
        TursoConfig::new("http://127.0.0.1:8080", "token")
            .validate()
            .is_err()
    );
    assert_eq!(
        format!("{:?}", TursoConfig::new("libsql://db.turso.io", "token")),
        "TursoConfig { url: \"[CONFIGURED]\", auth_token: \"[REDACTED]\", allow_insecure_loopback: false, offline_path: None }"
    );
}

#[tokio::test]
async fn transactions_are_bounded_and_remote_stores_are_online() {
    let remote = TursoStore::connect(TursoConfig::new("libsql://db.turso.io", "token"))
        .await
        .expect("a remote client is built without connecting");
    assert!(!remote.is_offline());

    let store = TursoStore::connect(TursoConfig::new("mock_local", ""))
        .await
        .unwrap();
    store
        .execute(statement("CREATE TABLE t (v BLOB)", vec![]))
        .await
        .unwrap();
    let insert = |bytes: usize| {
        statement(
            "INSERT INTO t VALUES (?1)",
            vec![TursoValue::Blob(vec![0; bytes])],
        )
    };
    let sql = "INSERT INTO t VALUES (?1)".len();
    let full = vec![insert(1024), insert(1024)];
    assert_eq!(store.transaction(full).await.unwrap(), [1, 1]);
    assert!(store.transaction(vec![insert(0); 1_024]).await.is_ok());
    assert!(store.transaction(vec![insert(0); 1_025]).await.is_err());
    // 16 MiB of SQL and parameters in total is accepted; one byte more is not.
    let mib = 1024 * 1024;
    let at_limit = vec![insert(8 * mib - sql), insert(8 * mib - sql)];
    assert!(store.transaction(at_limit).await.is_ok());
    let over = vec![insert(8 * mib - sql), insert(8 * mib - sql + 1)];
    assert!(store.transaction(over).await.is_err());
}

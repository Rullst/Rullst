#![cfg(feature = "turso")]

mod support;

use std::time::Duration;

use rullst_orm::polyglot::{
    PolyglotError, TursoConfig, TursoMigration, TursoQueryLimit, TursoStatement, TursoStore,
    TursoValue,
};
use testcontainers::GenericImage;
use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;

#[derive(Debug, Clone, PartialEq, rullst_orm::Orm)]
#[orm(table = "remote_users", backend = "turso")]
struct RemoteUser {
    id: i64,
    name: String,
    active: bool,
}

#[derive(Debug, Clone, PartialEq, rullst_orm::Orm)]
#[orm(table = "remote_tags", backend = "turso")]
struct RemoteTag {
    id: String,
    label: String,
}

#[derive(Debug, Clone, PartialEq, rullst_orm::Orm)]
#[orm(table = "remote_notes", backend = "turso")]
struct RemoteNote {
    id: String,
    body: String,
}

fn statement(sql: &str, parameters: Vec<TursoValue>) -> TursoStatement {
    TursoStatement::new(sql, parameters).expect("valid test statement")
}

#[tokio::test]
async fn test_matrix_turso_remote_sql_contract() {
    let container = match GenericImage::new("ghcr.io/tursodatabase/libsql-server", "v0.24.32")
        .with_exposed_port(8080.tcp())
        .start()
        .await
    {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("Turso/libSQL", error);
            return;
        }
    };
    let host = container
        .get_host()
        .await
        .expect("libSQL container host should be available");
    let port = container
        .get_host_port_ipv4(8080)
        .await
        .expect("libSQL container HTTP port should be available");
    let store = TursoStore::connect(
        TursoConfig::new(format!("http://{host}:{port}"), "").allow_insecure_loopback(),
    )
    .await
    .expect("official Hrana remote protocol should initialize");
    assert!(!store.is_offline());

    let migration = TursoMigration::new(
        "m20260829_edge_events",
        vec![statement(
            "CREATE TABLE edge_events (sequence INTEGER PRIMARY KEY, label TEXT NOT NULL)",
            vec![],
        )],
    )
    .expect("valid Turso migration");
    let mut last_error = None;
    for _ in 0..60 {
        match store.migrate(vec![migration.clone()]).await {
            Ok(_) => {
                last_error = None;
                break;
            }
            Err(error) => {
                last_error = Some(error);
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
    assert!(
        last_error.is_none(),
        "libSQL server did not become ready: {last_error:?}"
    );
    let repeated = store
        .migrate(vec![migration])
        .await
        .expect("remote migration should be idempotent");
    assert_eq!(repeated.skipped, vec!["m20260829_edge_events"]);

    store
        .transaction(vec![
            statement(
                "INSERT INTO edge_events VALUES (?1, ?2)",
                vec![
                    TursoValue::Integer(1),
                    TursoValue::Text("created remotely".into()),
                ],
            ),
            statement(
                "UPDATE edge_events SET label = ?1 WHERE sequence = ?2",
                vec![
                    TursoValue::Text("updated remotely".into()),
                    TursoValue::Integer(1),
                ],
            ),
        ])
        .await
        .expect("remote transaction should commit");

    let rows = store
        .query(
            statement(
                "SELECT sequence, label FROM edge_events WHERE sequence = ?1",
                vec![TursoValue::Integer(1)],
            ),
            TursoQueryLimit::new(10).expect("bounded query"),
        )
        .await
        .expect("remote parameterized query should succeed");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("sequence"), Some(&TursoValue::Integer(1)));
    assert_eq!(
        rows[0].get("label"),
        Some(&TursoValue::Text("updated remotely".into()))
    );

    let failed = store
        .transaction(vec![
            statement(
                "INSERT INTO edge_events VALUES (?1, ?2)",
                vec![
                    TursoValue::Integer(2),
                    TursoValue::Text("must roll back".into()),
                ],
            ),
            statement(
                "INSERT INTO edge_events VALUES (?1, ?2)",
                vec![
                    TursoValue::Integer(1),
                    TursoValue::Text("duplicate key".into()),
                ],
            ),
        ])
        .await;
    assert!(
        matches!(failed, Err(PolyglotError::Conflict)),
        "duplicate key transaction must fail as a conflict: {failed:?}"
    );
    let duplicate = store
        .execute(statement(
            "INSERT INTO edge_events VALUES (?1, ?2)",
            vec![TursoValue::Integer(1), TursoValue::Text("again".into())],
        ))
        .await;
    assert!(
        matches!(duplicate, Err(PolyglotError::Conflict)),
        "{duplicate:?}"
    );
    let missing_label = store
        .execute(statement(
            "INSERT INTO edge_events VALUES (?1, NULL)",
            vec![TursoValue::Integer(3)],
        ))
        .await;
    assert!(
        matches!(&missing_label, Err(PolyglotError::Driver { message, .. }) if message.contains("NOT NULL constraint failed")),
        "{missing_label:?}"
    );
    let rolled_back = store
        .query(
            statement(
                "SELECT sequence FROM edge_events WHERE sequence = ?1",
                vec![TursoValue::Integer(2)],
            ),
            TursoQueryLimit::new(1).expect("bounded rollback query"),
        )
        .await
        .expect("rollback verification query should succeed");
    assert!(rolled_back.is_empty(), "failed batch must roll back fully");

    // libSQL server returns blob cells as unpadded base64, so lengths that
    // are not a multiple of three exercise the padding-free decoder.
    store
        .execute(statement(
            "CREATE TABLE edge_blobs (id INTEGER PRIMARY KEY, payload BLOB NOT NULL)",
            vec![],
        ))
        .await
        .expect("blob table should be created remotely");
    for (id, payload) in [
        (1, vec![7_u8]),
        (2, vec![7, 8]),
        (3, vec![7, 8, 9]),
        (4, (0..16).collect::<Vec<u8>>()),
    ] {
        store
            .execute(statement(
                "INSERT INTO edge_blobs VALUES (?1, ?2)",
                vec![TursoValue::Integer(id), TursoValue::Blob(payload.clone())],
            ))
            .await
            .expect("remote blob insert");
        let rows = store
            .query(
                statement(
                    "SELECT payload FROM edge_blobs WHERE id = ?1",
                    vec![TursoValue::Integer(id)],
                ),
                TursoQueryLimit::new(1).expect("bounded blob query"),
            )
            .await
            .expect("remote blob cell should decode");
        assert_eq!(rows[0].get("payload"), Some(&TursoValue::Blob(payload)));
    }

    store
        .execute(statement(
            "CREATE TABLE remote_users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, active INTEGER NOT NULL)",
            vec![],
        ))
        .await
        .expect("typed-model table should be created remotely");
    let repository = store.models::<RemoteUser>();
    let mut user = RemoteUser {
        id: 0,
        name: "remote typed model".to_owned(),
        active: true,
    };
    repository
        .save(&mut user)
        .await
        .expect("typed remote insert should return an ID");
    assert!(user.id > 0);
    assert_eq!(
        repository
            .find(user.id)
            .await
            .expect("typed remote find")
            .expect("inserted remote model"),
        user
    );
    user.active = false;
    repository
        .save(&mut user)
        .await
        .expect("typed remote update");
    repository.delete(&user).await.expect("typed remote delete");

    // An empty text key comes from the column's DEFAULT; without one the
    // insert is rolled back and nothing is written. A duplicate key or
    // UNIQUE value is a conflict.
    store
        .execute(statement(
            "CREATE TABLE remote_tags (id TEXT PRIMARY KEY, label TEXT NOT NULL UNIQUE)",
            vec![],
        ))
        .await
        .expect("text-key table should be created remotely");
    let tags = store.models::<RemoteTag>();
    let mut unset = RemoteTag {
        id: String::new(),
        label: "no key".to_owned(),
    };
    let refused = tags.save(&mut unset).await;
    assert!(
        matches!(
            refused,
            Err(PolyglotError::InvalidIdentifier {
                kind: "Turso model primary key",
                ..
            })
        ),
        "{refused:?}"
    );
    assert_eq!(tag_count(&store).await, Some(TursoValue::Integer(0)));
    let mut rust = RemoteTag {
        id: "rust".to_owned(),
        label: "Rust".to_owned(),
    };
    tags.create(&mut rust).await.expect("app-assigned text key");
    for (id, label) in [("rust", "Other"), ("go", "Rust")] {
        let mut duplicate = RemoteTag {
            id: id.to_owned(),
            label: label.to_owned(),
        };
        let conflict = tags.create(&mut duplicate).await;
        assert!(
            matches!(conflict, Err(PolyglotError::Conflict)),
            "{id} {label}: {conflict:?}"
        );
    }
    assert_eq!(tag_count(&store).await, Some(TursoValue::Integer(1)));

    store
        .execute(statement(
            "CREATE TABLE remote_notes (id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))), body TEXT NOT NULL)",
            vec![],
        ))
        .await
        .expect("defaulted text-key table should be created remotely");
    let notes = store.models::<RemoteNote>();
    let mut note = RemoteNote {
        id: String::new(),
        body: "generated remotely".to_owned(),
    };
    notes
        .save(&mut note)
        .await
        .expect("a DEFAULT text key should be generated and returned");
    assert_eq!(note.id.len(), 32, "{}", note.id);
    assert_eq!(
        notes.find(note.id.clone()).await.expect("remote find"),
        Some(note)
    );
}

async fn tag_count(store: &TursoStore) -> Option<TursoValue> {
    store
        .query(
            statement("SELECT COUNT(*) AS c FROM remote_tags", vec![]),
            TursoQueryLimit::new(1).expect("bounded count"),
        )
        .await
        .expect("remote count")[0]
        .get("c")
        .cloned()
}

#![cfg(feature = "turso")]

mod support;

use std::time::Duration;

use rullst_orm::polyglot::{
    TursoConfig, TursoMigration, TursoQueryLimit, TursoStatement, TursoStore, TursoValue,
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
    .expect("official libSQL remote driver should initialize");
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
}

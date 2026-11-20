//! The idle-connection ping of the server matrices' `Orm::init` pool.

#![allow(dead_code)]

use std::time::Duration;

use rullst_orm::{Orm, RullstDatabase};
use sqlx::Connection;

/// Ends every pooled connection from a separate connection while they are
/// idle, waits past the pool's 500 ms idle window and runs a query on the
/// pool. The query succeeds on a new connection: the pool pinged the dead
/// ones, closed them and opened another instead of handing one out.
pub async fn exercise_idle_ping_replaces_ended_connections(database_url: &str) {
    let pool = Orm::pool().expect("initialized pool");
    let postgres = Orm::driver().expect("initialized driver") == "postgres";
    let (connection_id, pooled_connections) = if postgres {
        (
            "SELECT CAST(pg_backend_pid() AS BIGINT)",
            "SELECT CAST(pid AS BIGINT) FROM pg_stat_activity \
             WHERE datname = current_database() AND backend_type = 'client backend' \
             AND pid <> pg_backend_pid()",
        )
    } else {
        (
            "SELECT CAST(CONNECTION_ID() AS SIGNED)",
            "SELECT CAST(id AS SIGNED) FROM information_schema.processlist \
             WHERE db = DATABASE() AND id <> CONNECTION_ID()",
        )
    };

    let before: i64 = sqlx::query_scalar(connection_id)
        .fetch_one(pool)
        .await
        .expect("query on the pool");
    let mut admin =
        <<RullstDatabase as sqlx::Database>::Connection as Connection>::connect(database_url)
            .await
            .expect("separate admin connection");
    let ended: Vec<i64> = sqlx::query_scalar(pooled_connections)
        .fetch_all(&mut admin)
        .await
        .expect("list the pool's connections");
    assert!(
        ended.contains(&before),
        "the pooled connection {before} must be among {ended:?}"
    );
    for id in &ended {
        if postgres {
            let terminated: bool =
                sqlx::query_scalar("SELECT pg_terminate_backend(CAST($1 AS INTEGER))")
                    .bind(*id)
                    .fetch_one(&mut admin)
                    .await
                    .expect("terminate a pooled connection");
            assert!(terminated, "backend {id} was not terminated");
        } else {
            sqlx::raw_sql(sqlx::AssertSqlSafe(format!("KILL {id}")))
                .execute(&mut admin)
                .await
                .expect("kill a pooled connection");
        }
    }

    tokio::time::sleep(Duration::from_millis(600)).await;
    let after: i64 = sqlx::query_scalar(connection_id)
        .fetch_one(pool)
        .await
        .expect("the pool must replace connections that ended while idle");
    assert!(
        !ended.contains(&after),
        "the pool handed out connection {after}, which was ended"
    );
    admin.close().await.expect("close the admin connection");
}

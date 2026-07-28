#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "stress_users")]
struct StressUser {
    pub id: i32,
    pub name: String,
}

#[tokio::test]
async fn test_pool_exhaustion_and_concurrency() {
    let db_file = "stress_suite.db";
    let _ = std::fs::remove_file(db_file);

    // Conexão com SQLite para o stress test
    // Forçamos max_connections = 1 no SQLite para forçar serialização de writes
    // e evitar 'database is locked' em 200 tarefas simultâneas, enquanto
    // ainda validamos se a pool prende (deadlock).
    Orm::init_with_options(&format!("sqlite:{}?mode=rwc", db_file), 1, 30)
        .await
        .expect("Orm::init_with_options");

    Schema::create("stress_users", |t: &mut Blueprint| {
        t.id();
        t.string("name").not_null();
    })
    .await
    .expect("create stress_users");

    let counter = Arc::new(AtomicU32::new(0));
    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    // Spawna 200 tarefas concorrentes. O pool padrão do sqlx tem 10 conexões max.
    // Se a lib prender conexões (ex: deadlock), isso vai travar.
    let concurrency_level = 200;

    for i in 0..concurrency_level {
        let counter_clone = Arc::clone(&counter);

        let handle = tokio::spawn(async move {
            let mut user = StressUser {
                id: 0,
                name: format!("Worker {}", i),
            };

            // 1. Acquire connection from pool & INSERT
            user.save().await.expect("Failed to save under stress");
            assert!(user.id > 0);

            // 2. Acquire connection from pool & SELECT
            let found = StressUser::find(user.id).await.unwrap().unwrap();
            assert_eq!(found.name, format!("Worker {}", i));

            // 3. Acquire connection from pool & UPDATE
            user.name = format!("Worker {} - done", i);
            user.save().await.expect("Failed to update under stress");

            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        handles.push(handle);
    }

    // Espera todas as tarefas concluírem
    for handle in handles {
        handle.await.expect("tokio task panicked");
    }

    assert_eq!(counter.load(Ordering::SeqCst), concurrency_level);

    let _ = std::fs::remove_file(db_file);
}

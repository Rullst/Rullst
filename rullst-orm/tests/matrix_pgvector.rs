#![cfg(all(feature = "pgvector", feature = "strict-postgres"))]

mod support;

use rullst_orm::{FromRow, Orm, Vector};
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{GenericImage, ImageExt};

#[derive(Clone, Debug, FromRow, Orm)]
#[orm(table = "vector_documents")]
struct VectorDocument {
    id: i32,
    title: String,
    embedding: Vector,
}

#[tokio::test]
async fn pgvector_helpers_pass_a_live_parameterized_lifecycle() {
    let container = match GenericImage::new(
        "pgvector/pgvector",
        "pg18@sha256:1d50c689b0a6511b9ea0a15615281c81a59fd04a08eb35057ec8646fb3a2118a",
    )
    .with_wait_for(WaitFor::message_on_stderr(
        "database system is ready to accept connections",
    ))
    .with_exposed_port(5432.tcp())
    .with_env_var("POSTGRES_DB", "postgres")
    .with_env_var("POSTGRES_USER", "postgres")
    .with_env_var("POSTGRES_PASSWORD", "postgres")
    .start()
    .await
    {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("PostgreSQL + pgvector", error);
            return;
        }
    };
    let host = container
        .get_host()
        .await
        .expect("pgvector host should be available");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("pgvector port should be available");
    Orm::init(&format!(
        "postgres://postgres:postgres@{host}:{port}/postgres"
    ))
    .await
    .expect("strict PostgreSQL pool should connect to pgvector");

    sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
        .execute(Orm::pool().expect("pgvector pool should be initialized"))
        .await
        .expect("vector extension should install");
    sqlx::query(
        "CREATE TABLE vector_documents (\
            id SERIAL PRIMARY KEY,\
            title TEXT NOT NULL,\
            embedding vector(3) NOT NULL\
        )",
    )
    .execute(Orm::pool().expect("pgvector pool should be initialized"))
    .await
    .expect("vector table should be created");

    let mut closest = VectorDocument {
        id: 0,
        title: "closest".to_string(),
        embedding: Vector::from(vec![1.0_f32, 0.0, 0.0]),
    };
    closest
        .save()
        .await
        .expect("typed pgvector document should insert");
    let mut farther = VectorDocument {
        id: 0,
        title: "farther".to_string(),
        embedding: Vector::from(vec![0.0_f32, 1.0, 0.0]),
    };
    farther
        .save()
        .await
        .expect("second typed pgvector document should insert");

    let results = VectorDocument::query()
        .order_by_l2_distance("embedding", vec![0.0, 1.0, 0.0])
        .where_similar("embedding", vec![1.0, 0.0, 0.0], 0.25)
        .get()
        .await
        .expect("parameterized vector query should execute");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, closest.id);
    assert_eq!(results[0].title, "closest");

    let cosine = VectorDocument::query()
        .order_by_cosine_distance("embedding", vec![0.0, 1.0, 0.0])
        .first()
        .await
        .expect("cosine query should execute")
        .expect("cosine query should return a document");
    assert_eq!(cosine.id, farther.id);
}

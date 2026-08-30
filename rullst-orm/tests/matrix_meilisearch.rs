#![cfg(feature = "scout-http")]

mod support;

use std::time::Duration;

use rullst_orm::{MeilisearchEngine, SearchEngine};
use serde_json::json;
use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{GenericImage, ImageExt};

#[tokio::test]
async fn meilisearch_adapter_passes_a_live_index_lifecycle() {
    let container = match GenericImage::new(
        "getmeili/meilisearch",
        "v1.53.1@sha256:81ffa96ce6e9a6769775d772742ed3da653ad34960cd6c3b43b98918c07db101",
    )
    .with_exposed_port(7700.tcp())
    .with_env_var("MEILI_NO_ANALYTICS", "true")
    .with_env_var("MEILI_ENV", "development")
    .start()
    .await
    {
        Ok(container) => container,
        Err(error) => {
            support::handle_container_start_error("Meilisearch", error);
            return;
        }
    };
    let host = container
        .get_host()
        .await
        .expect("Meilisearch host should be available");
    let port = container
        .get_host_port_ipv4(7700)
        .await
        .expect("Meilisearch port should be available");
    let engine = MeilisearchEngine::local(format!("http://{host}:{port}"))
        .expect("loopback Meilisearch endpoint should be accepted");
    assert!(!engine.is_offline());

    let mut last_error = None;
    for _ in 0..60 {
        match engine
            .update(
                "search_articles",
                1,
                json!({"title": "Rullst durable search"}),
            )
            .await
        {
            Ok(()) => {
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
        "Meilisearch did not become ready: {last_error:?}"
    );
    engine
        .update("search_articles", 2, json!({"title": "Another document"}))
        .await
        .expect("index second document");
    assert_eq!(
        engine
            .search("search_articles", "durable")
            .await
            .expect("search live index"),
        vec![1]
    );
    engine
        .delete("search_articles", 1)
        .await
        .expect("delete indexed document");
    assert!(
        engine
            .search("search_articles", "durable")
            .await
            .expect("search after delete")
            .is_empty()
    );
}

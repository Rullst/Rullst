#![cfg(feature = "scout-http")]

use rullst_orm::{AlgoliaEngine, ElasticsearchEngine, MeilisearchEngine, SearchEngine};
use serde_json::json;

#[tokio::test]
async fn provider_adapters_offer_the_same_deterministic_offline_contract() {
    let meili = MeilisearchEngine::new("https://search.example.test", "")
        .expect("empty Meilisearch credentials select offline mode");
    let elastic = ElasticsearchEngine::new("", "live-looking-key")
        .expect("empty Elasticsearch endpoint selects offline mode");
    let algolia = AlgoliaEngine::new("mock_application", "mock_key")
        .expect("mock Algolia credentials select offline mode");
    assert!(meili.is_offline());
    assert!(elastic.is_offline());
    assert!(algolia.is_offline());

    exercise_offline(&meili).await;
    exercise_offline(&elastic).await;
    exercise_offline(&algolia).await;
    assert!(
        meili
            .update("articles", 3, json!({"body": "x".repeat(1_048_577)}))
            .await
            .is_err()
    );
    assert!(meili.search("articles", &"x".repeat(1_025)).await.is_err());
}

#[test]
fn live_provider_configuration_fails_closed() {
    assert!(MeilisearchEngine::new("http://search.example.test", "live-key").is_err());
    assert!(MeilisearchEngine::new("https://user@search.example.test", "live-key").is_err());
    assert!(MeilisearchEngine::local("https://search.example.test").is_err());
    assert!(ElasticsearchEngine::new("https://search.example.test/path", "live-key").is_err());
    assert!(ElasticsearchEngine::new("https://search.example.test", "bad\nkey").is_err());
    assert!(AlgoliaEngine::new("bad_application!", "live-key").is_err());
    assert!(
        AlgoliaEngine::with_endpoint("http://search.example.test", "APP123", "live-key").is_err()
    );
}

async fn exercise_offline(engine: &impl SearchEngine) {
    engine
        .update("articles", 2, json!({"title": "Rust search"}))
        .await
        .expect("insert offline search document");
    engine
        .update("articles", 1, json!({"title": "Rullst framework"}))
        .await
        .expect("insert second offline search document");
    assert_eq!(
        engine
            .search("articles", "rullst")
            .await
            .expect("search offline documents"),
        vec![1]
    );
    assert_eq!(
        engine
            .search("articles", "")
            .await
            .expect("list offline documents"),
        vec![1, 2]
    );
    engine
        .delete("articles", 1)
        .await
        .expect("delete offline search document");
    assert_eq!(
        engine
            .search("articles", "")
            .await
            .expect("list remaining offline documents"),
        vec![2]
    );
}

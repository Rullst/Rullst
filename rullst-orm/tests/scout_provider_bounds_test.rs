//! Task polling, hit limits and live/offline selection of the HTTP Scout
//! providers, against a loopback fixture.
#![cfg(feature = "scout-http")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use rullst_orm::{AlgoliaEngine, ElasticsearchEngine, MeilisearchEngine, SearchEngine};
use serde_json::{Value, json};

/// Task polls per task id, and the states each poll answers with.
#[derive(Default)]
struct Tasks {
    polls: Mutex<HashMap<u64, usize>>,
}

impl Tasks {
    fn poll(&self, task: u64) -> usize {
        let mut polls = self.polls.lock().unwrap();
        let count = polls.entry(task).or_default();
        *count += 1;
        *count
    }

    fn polls(&self, task: u64) -> usize {
        self.polls.lock().unwrap().get(&task).copied().unwrap_or(0)
    }
}

fn hits(count: usize, key: &str) -> Vec<Value> {
    (1..=count)
        .map(|id| json!({ key: id.to_string() }))
        .collect()
}

fn hit_count(index: &str) -> usize {
    match index {
        "full" => 1_000,
        "over" => 1_001,
        _ => 1,
    }
}

/// Task 1 is pending once, task 2 has an unknown state, task 3 failed.
fn state_of(task: u64, poll: usize, pending: &str, done: &str) -> String {
    match (task, poll) {
        (1, 1) => pending.to_string(),
        (1, _) => done.to_string(),
        (2, _) => "mystery".to_string(),
        _ => "failed".to_string(),
    }
}

async fn algolia_upsert(Path((index, _id)): Path<(String, i32)>) -> Json<Value> {
    let task = match index.as_str() {
        "pending" => 1,
        "unknown" => 2,
        _ => 3,
    };
    Json(json!({ "taskID": task }))
}

async fn algolia_task(
    State(tasks): State<Arc<Tasks>>,
    Path((_index, task)): Path<(String, u64)>,
) -> Json<Value> {
    let poll = tasks.poll(task);
    Json(json!({ "status": state_of(task, poll, "notPublished", "published") }))
}

async fn algolia_search(Path(index): Path<String>) -> Json<Value> {
    Json(json!({ "hits": hits(hit_count(&index), "objectID") }))
}

async fn elastic_search(Path(index): Path<String>) -> Json<Value> {
    Json(json!({ "hits": { "hits": hits(hit_count(&index), "_id") } }))
}

async fn meili_documents(Path(index): Path<String>) -> Json<Value> {
    let task = match index.as_str() {
        "pending" => 1,
        "unknown" => 2,
        _ => 3,
    };
    Json(json!({ "taskUid": task }))
}

async fn meili_task(State(tasks): State<Arc<Tasks>>, Path(task): Path<u64>) -> Json<Value> {
    // Meilisearch task ids are offset so they do not share Algolia's counters.
    let poll = tasks.poll(task + 100);
    Json(json!({ "status": state_of(task, poll, "processing", "succeeded") }))
}

async fn meili_search(Path(index): Path<String>) -> Json<Value> {
    Json(json!({ "hits": hits(hit_count(&index), "id") }))
}

#[tokio::test]
async fn providers_poll_tasks_bound_hits_and_select_live_mode() {
    let tasks = Arc::new(Tasks::default());
    let router = Router::new()
        .route("/1/indexes/{index}/{id}", put(algolia_upsert))
        .route("/1/indexes/{index}/task/{task}", get(algolia_task))
        .route("/1/indexes/{index}/query", post(algolia_search))
        .route("/{index}/_search", post(elastic_search))
        .route("/indexes/{index}/documents", post(meili_documents))
        .route("/tasks/{task}", get(meili_task))
        .route("/indexes/{index}/search", post(meili_search))
        .with_state(tasks.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });

    let algolia = AlgoliaEngine::with_endpoint(&endpoint, "APP123", "algolia-key").unwrap();
    let elastic = ElasticsearchEngine::local(&endpoint).unwrap();
    let meili = MeilisearchEngine::local(&endpoint).unwrap();
    assert!(!algolia.is_offline() && !elastic.is_offline() && !meili.is_offline());
    assert!(AlgoliaEngine::offline().is_offline());
    assert!(ElasticsearchEngine::offline().is_offline());
    assert!(MeilisearchEngine::offline().is_offline());

    // A pending task is polled again until it completes.
    algolia
        .update("pending", 1, json!({}))
        .await
        .expect("published");
    assert_eq!(tasks.polls(1), 2);
    meili
        .update("pending", 1, json!({}))
        .await
        .expect("succeeded");
    assert_eq!(tasks.polls(101), 2);
    // Unknown and failed states end the wait with an error.
    assert!(algolia.update("unknown", 1, json!({})).await.is_err());
    assert!(meili.update("unknown", 1, json!({})).await.is_err());
    assert_eq!(
        meili
            .update("failed", 1, json!({}))
            .await
            .unwrap_err()
            .to_string(),
        "Internal error: Meilisearch indexing task did not succeed"
    );

    for engine in [&algolia as &(dyn SearchEngine + Sync), &elastic, &meili] {
        assert_eq!(engine.search("full", "q").await.unwrap().len(), 1_000);
        assert!(engine.search("over", "q").await.is_err());
    }
    server.abort();
}

#[test]
fn algolia_application_ids_are_bounded_ascii() {
    let endpoint = "https://algolia.example.com";
    assert!(AlgoliaEngine::with_endpoint(endpoint, "A".repeat(64), "key").is_ok());
    for invalid in ["A".repeat(65), "APP 1".to_string(), "APP_1".to_string()] {
        assert!(
            AlgoliaEngine::with_endpoint(endpoint, invalid.clone(), "key").is_err(),
            "{invalid}"
        );
    }
}

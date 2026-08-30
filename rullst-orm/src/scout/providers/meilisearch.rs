use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use url::Url;

use super::common::{
    MAX_SEARCH_HITS, client, endpoint, join, json_response, mock_requested, parse_positive_id,
    request_error, validate_credential,
};
use crate::Error;
use crate::scout::{
    MockSearchEngine, SearchEngine, document_with_id, validate_index, validate_query,
};

/// Bounded Meilisearch Scout adapter with a deterministic offline mode.
pub struct MeilisearchEngine {
    mode: Mode,
}

enum Mode {
    Offline(MockSearchEngine),
    Live(Live),
}

struct Live {
    client: Client,
    endpoint: Url,
    api_key: Option<String>,
}

impl MeilisearchEngine {
    /// Builds a remote adapter. Empty or `mock_*` credentials select offline mode.
    pub fn new(
        endpoint_value: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<Self, Error> {
        let endpoint_value = endpoint_value.into();
        let api_key = api_key.into();
        if mock_requested(&[&endpoint_value, &api_key]) {
            return Ok(Self::offline());
        }
        validate_credential("Meilisearch API key", &api_key)?;
        Ok(Self {
            mode: Mode::Live(Live {
                client: client()?,
                endpoint: endpoint(&endpoint_value)?,
                api_key: Some(api_key),
            }),
        })
    }

    /// Connects without a key only to a loopback development service.
    pub fn local(endpoint_value: impl Into<String>) -> Result<Self, Error> {
        let endpoint_value = endpoint_value.into();
        let endpoint = endpoint(&endpoint_value)?;
        if endpoint.scheme() != "http" {
            return Err(Error::Validation(
                "keyless Meilisearch is limited to loopback HTTP development".to_string(),
            ));
        }
        Ok(Self {
            mode: Mode::Live(Live {
                client: client()?,
                endpoint,
                api_key: None,
            }),
        })
    }

    /// Creates the deterministic offline adapter directly.
    pub fn offline() -> Self {
        Self {
            mode: Mode::Offline(MockSearchEngine::new()),
        }
    }

    pub fn is_offline(&self) -> bool {
        matches!(self.mode, Mode::Offline(_))
    }
}

impl Live {
    fn authorize(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(api_key) = &self.api_key {
            request.bearer_auth(api_key)
        } else {
            request
        }
    }

    async fn wait_for_task(&self, task_uid: u64) -> Result<(), Error> {
        let url = join(&self.endpoint, &format!("tasks/{task_uid}"))?;
        for _ in 0..100 {
            let response = self
                .authorize(self.client.get(url.clone()))
                .send()
                .await
                .map_err(|_| request_error("Meilisearch"))?;
            let payload = json_response("Meilisearch", response, &[StatusCode::OK]).await?;
            match payload.get("status").and_then(Value::as_str) {
                Some("succeeded") => return Ok(()),
                Some("failed" | "canceled") => {
                    return Err(Error::Internal(
                        "Meilisearch indexing task did not succeed".to_string(),
                    ));
                }
                Some("enqueued" | "processing") => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                _ => {
                    return Err(Error::Validation(
                        "Meilisearch returned an unknown task state".to_string(),
                    ));
                }
            }
        }
        Err(Error::Internal(
            "Meilisearch indexing task exceeded the bounded wait".to_string(),
        ))
    }

    async fn finish_task(&self, payload: &Value) -> Result<(), Error> {
        let task_uid = payload
            .get("taskUid")
            .and_then(Value::as_u64)
            .ok_or_else(|| Error::Validation("Meilisearch response omitted taskUid".to_string()))?;
        self.wait_for_task(task_uid).await
    }
}

#[async_trait]
impl SearchEngine for MeilisearchEngine {
    async fn update(&self, table: &str, id: i32, payload: Value) -> Result<(), Error> {
        validate_index(table)?;
        let payload = document_with_id(id, payload)?;
        let Mode::Live(live) = &self.mode else {
            if let Mode::Offline(mock) = &self.mode {
                return mock.update(table, id, payload).await;
            }
            return Ok(());
        };
        let mut url = join(&live.endpoint, &format!("indexes/{table}/documents"))?;
        url.query_pairs_mut().append_pair("primaryKey", "id");
        let response = live
            .authorize(live.client.post(url))
            .json(&[payload])
            .send()
            .await
            .map_err(|_| request_error("Meilisearch"))?;
        let task = json_response(
            "Meilisearch",
            response,
            &[StatusCode::OK, StatusCode::ACCEPTED],
        )
        .await?;
        live.finish_task(&task).await
    }

    async fn delete(&self, table: &str, id: i32) -> Result<(), Error> {
        validate_index(table)?;
        if id <= 0 {
            return Err(Error::Validation(
                "Scout document id must be positive".to_string(),
            ));
        }
        let Mode::Live(live) = &self.mode else {
            if let Mode::Offline(mock) = &self.mode {
                return mock.delete(table, id).await;
            }
            return Ok(());
        };
        let url = join(&live.endpoint, &format!("indexes/{table}/documents/{id}"))?;
        let response = live
            .authorize(live.client.delete(url))
            .send()
            .await
            .map_err(|_| request_error("Meilisearch"))?;
        let task = json_response(
            "Meilisearch",
            response,
            &[StatusCode::OK, StatusCode::ACCEPTED],
        )
        .await?;
        live.finish_task(&task).await
    }

    async fn search(&self, table: &str, query: &str) -> Result<Vec<i32>, Error> {
        validate_index(table)?;
        validate_query(query)?;
        let Mode::Live(live) = &self.mode else {
            if let Mode::Offline(mock) = &self.mode {
                return mock.search(table, query).await;
            }
            return Ok(Vec::new());
        };
        let url = join(&live.endpoint, &format!("indexes/{table}/search"))?;
        let response = live
            .authorize(live.client.post(url))
            .json(&json!({"q": query, "limit": MAX_SEARCH_HITS}))
            .send()
            .await
            .map_err(|_| request_error("Meilisearch"))?;
        let payload = json_response("Meilisearch", response, &[StatusCode::OK]).await?;
        let hits = payload
            .get("hits")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Validation("Meilisearch response omitted hits".to_string()))?;
        if hits.len() > MAX_SEARCH_HITS {
            return Err(Error::Validation(
                "Meilisearch response exceeds the hit limit".to_string(),
            ));
        }
        hits.iter()
            .map(|hit| {
                hit.get("id")
                    .ok_or_else(|| Error::Validation("Meilisearch hit omitted id".to_string()))
                    .and_then(|id| parse_positive_id(id, "Meilisearch"))
            })
            .collect()
    }
}

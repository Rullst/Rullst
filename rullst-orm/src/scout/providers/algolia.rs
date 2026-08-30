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

/// Bounded Algolia Scout adapter with a deterministic offline mode.
pub struct AlgoliaEngine {
    mode: Mode,
}

enum Mode {
    Offline(MockSearchEngine),
    Live(Live),
}

struct Live {
    client: Client,
    endpoint: Url,
    application_id: String,
    api_key: String,
}

impl AlgoliaEngine {
    /// Builds an Algolia adapter. Empty or `mock_*` credentials select offline mode.
    pub fn new(
        application_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<Self, Error> {
        let application_id = application_id.into();
        let api_key = api_key.into();
        if mock_requested(&[&application_id, &api_key]) {
            return Ok(Self::offline());
        }
        let endpoint = Url::parse(&format!(
            "https://{}.algolia.net/",
            application_id.to_ascii_lowercase()
        ))
        .map_err(|_| Error::Internal("could not construct Algolia endpoint".to_string()))?;
        Self::from_endpoint(endpoint, application_id, api_key)
    }

    /// Builds an adapter for an explicit Algolia-compatible proxy origin.
    pub fn with_endpoint(
        endpoint_value: impl Into<String>,
        application_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<Self, Error> {
        let endpoint_value = endpoint_value.into();
        let application_id = application_id.into();
        let api_key = api_key.into();
        if mock_requested(&[&endpoint_value, &application_id, &api_key]) {
            return Ok(Self::offline());
        }
        Self::from_endpoint(endpoint(&endpoint_value)?, application_id, api_key)
    }

    fn from_endpoint(
        endpoint: Url,
        application_id: String,
        api_key: String,
    ) -> Result<Self, Error> {
        if mock_requested(&[&application_id, &api_key]) {
            return Ok(Self::offline());
        }
        if application_id.is_empty()
            || application_id.len() > 64
            || !application_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(Error::Validation(
                "Algolia application id is outside its ASCII bound".to_string(),
            ));
        }
        validate_credential("Algolia API key", &api_key)?;
        Ok(Self {
            mode: Mode::Live(Live {
                client: client()?,
                endpoint,
                application_id,
                api_key,
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
        request
            .header("x-algolia-application-id", &self.application_id)
            .header("x-algolia-api-key", &self.api_key)
    }

    async fn wait_for_task(&self, table: &str, task_id: u64) -> Result<(), Error> {
        let url = join(&self.endpoint, &format!("1/indexes/{table}/task/{task_id}"))?;
        for _ in 0..100 {
            let response = self
                .authorize(self.client.get(url.clone()))
                .send()
                .await
                .map_err(|_| request_error("Algolia"))?;
            let payload = json_response("Algolia", response, &[StatusCode::OK]).await?;
            match payload.get("status").and_then(Value::as_str) {
                Some("published") => return Ok(()),
                Some("notPublished") => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                _ => {
                    return Err(Error::Validation(
                        "Algolia returned an unknown task state".to_string(),
                    ));
                }
            }
        }
        Err(Error::Internal(
            "Algolia indexing task exceeded the bounded wait".to_string(),
        ))
    }

    async fn finish_task(&self, table: &str, payload: &Value) -> Result<(), Error> {
        let task_id = payload
            .get("taskID")
            .and_then(Value::as_u64)
            .ok_or_else(|| Error::Validation("Algolia response omitted taskID".to_string()))?;
        self.wait_for_task(table, task_id).await
    }
}

#[async_trait]
impl SearchEngine for AlgoliaEngine {
    async fn update(&self, table: &str, id: i32, payload: Value) -> Result<(), Error> {
        validate_index(table)?;
        let mut payload = document_with_id(id, payload)?;
        let object = payload.as_object_mut().ok_or_else(|| {
            Error::Internal("validated Scout payload stopped being an object".to_string())
        })?;
        object.insert("objectID".to_string(), Value::String(id.to_string()));
        let Mode::Live(live) = &self.mode else {
            if let Mode::Offline(mock) = &self.mode {
                return mock.update(table, id, payload).await;
            }
            return Ok(());
        };
        let url = join(&live.endpoint, &format!("1/indexes/{table}/{id}"))?;
        let response = live
            .authorize(live.client.put(url))
            .json(&payload)
            .send()
            .await
            .map_err(|_| request_error("Algolia"))?;
        let task =
            json_response("Algolia", response, &[StatusCode::OK, StatusCode::CREATED]).await?;
        live.finish_task(table, &task).await
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
        let url = join(&live.endpoint, &format!("1/indexes/{table}/{id}"))?;
        let response = live
            .authorize(live.client.delete(url))
            .send()
            .await
            .map_err(|_| request_error("Algolia"))?;
        let task =
            json_response("Algolia", response, &[StatusCode::OK, StatusCode::ACCEPTED]).await?;
        live.finish_task(table, &task).await
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
        let url = join(&live.endpoint, &format!("1/indexes/{table}/query"))?;
        let response = live
            .authorize(live.client.post(url))
            .json(&json!({
                "query": query,
                "hitsPerPage": MAX_SEARCH_HITS,
                "attributesToRetrieve": ["objectID"]
            }))
            .send()
            .await
            .map_err(|_| request_error("Algolia"))?;
        let payload = json_response("Algolia", response, &[StatusCode::OK]).await?;
        let hits = payload
            .get("hits")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Validation("Algolia response omitted hits".to_string()))?;
        if hits.len() > MAX_SEARCH_HITS {
            return Err(Error::Validation(
                "Algolia response exceeds the hit limit".to_string(),
            ));
        }
        hits.iter()
            .map(|hit| {
                hit.get("objectID")
                    .ok_or_else(|| Error::Validation("Algolia hit omitted objectID".to_string()))
                    .and_then(|id| parse_positive_id(id, "Algolia"))
            })
            .collect()
    }
}

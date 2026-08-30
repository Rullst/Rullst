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

/// Bounded Elasticsearch Scout adapter with a deterministic offline mode.
pub struct ElasticsearchEngine {
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

impl ElasticsearchEngine {
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
        validate_credential("Elasticsearch API key", &api_key)?;
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
                "keyless Elasticsearch is limited to loopback HTTP development".to_string(),
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
            request.header("authorization", format!("ApiKey {api_key}"))
        } else {
            request
        }
    }
}

#[async_trait]
impl SearchEngine for ElasticsearchEngine {
    async fn update(&self, table: &str, id: i32, payload: Value) -> Result<(), Error> {
        validate_index(table)?;
        let payload = document_with_id(id, payload)?;
        let Mode::Live(live) = &self.mode else {
            if let Mode::Offline(mock) = &self.mode {
                return mock.update(table, id, payload).await;
            }
            return Ok(());
        };
        let mut url = join(&live.endpoint, &format!("{table}/_doc/{id}"))?;
        url.query_pairs_mut().append_pair("refresh", "wait_for");
        let response = live
            .authorize(live.client.put(url))
            .json(&payload)
            .send()
            .await
            .map_err(|_| request_error("Elasticsearch"))?;
        json_response(
            "Elasticsearch",
            response,
            &[StatusCode::OK, StatusCode::CREATED],
        )
        .await?;
        Ok(())
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
        let mut url = join(&live.endpoint, &format!("{table}/_doc/{id}"))?;
        url.query_pairs_mut().append_pair("refresh", "wait_for");
        let response = live
            .authorize(live.client.delete(url))
            .send()
            .await
            .map_err(|_| request_error("Elasticsearch"))?;
        json_response(
            "Elasticsearch",
            response,
            &[StatusCode::OK, StatusCode::NOT_FOUND],
        )
        .await?;
        Ok(())
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
        let url = join(&live.endpoint, &format!("{table}/_search"))?;
        let response = live
            .authorize(live.client.post(url))
            .json(&json!({
                "size": MAX_SEARCH_HITS,
                "_source": false,
                "query": {"simple_query_string": {"query": query}}
            }))
            .send()
            .await
            .map_err(|_| request_error("Elasticsearch"))?;
        let payload = json_response("Elasticsearch", response, &[StatusCode::OK]).await?;
        let hits = payload
            .pointer("/hits/hits")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Validation("Elasticsearch response omitted hits".to_string()))?;
        if hits.len() > MAX_SEARCH_HITS {
            return Err(Error::Validation(
                "Elasticsearch response exceeds the hit limit".to_string(),
            ));
        }
        hits.iter()
            .map(|hit| {
                hit.get("_id")
                    .ok_or_else(|| Error::Validation("Elasticsearch hit omitted _id".to_string()))
                    .and_then(|id| parse_positive_id(id, "Elasticsearch"))
            })
            .collect()
    }
}

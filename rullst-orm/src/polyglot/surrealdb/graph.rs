use async_trait::async_trait;
use serde_json::Value;

use super::PolyglotError;

const MAX_GRAPH_QUERY_BYTES: usize = 64 * 1024;
const MAX_GRAPH_ROWS: u32 = 1_000;

/// A validated read-only ISO GQL query with a framework-controlled limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphQuery {
    query: String,
    pub(super) limit: u32,
}

impl GraphQuery {
    /// Accepts one `MATCH` query, rejects mutation tokens, and appends `LIMIT`.
    pub fn read_only(query: impl Into<String>, limit: u32) -> Result<Self, PolyglotError> {
        let query = query.into();
        let trimmed = query.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_GRAPH_QUERY_BYTES {
            return Err(invalid_graph_query(
                "query must contain 1-65536 UTF-8 bytes",
            ));
        }
        if limit == 0 || limit > MAX_GRAPH_ROWS {
            return Err(invalid_graph_query("limit must be between 1 and 1000"));
        }
        if trimmed.contains(';') {
            return Err(invalid_graph_query(
                "multiple statements and semicolons are not accepted",
            ));
        }
        let tokens = graph_tokens(trimmed);
        if tokens.first().map(String::as_str) != Some("MATCH") {
            return Err(invalid_graph_query(
                "read-only graph queries must start with MATCH",
            ));
        }
        if tokens.iter().any(|token| {
            matches!(
                token.as_str(),
                "INSERT" | "SET" | "REMOVE" | "DELETE" | "LIMIT"
            )
        }) {
            return Err(invalid_graph_query(
                "mutation tokens and caller-provided LIMIT are not accepted",
            ));
        }
        Ok(Self {
            query: trimmed.to_owned(),
            limit,
        })
    }

    pub(super) fn bounded_query(&self) -> String {
        format!("{} LIMIT {}", self.query, self.limit)
    }
}

/// Read-only graph query boundary for SurrealDB's ISO GQL endpoint.
#[async_trait]
pub trait GraphRepository: Send + Sync {
    /// Runs a validated, bounded graph query and returns its JSON rows.
    async fn query_graph(&self, query: &GraphQuery) -> Result<Vec<Value>, PolyglotError>;
}

fn graph_tokens(query: &str) -> Vec<String> {
    query
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_uppercase)
        .collect()
}

fn invalid_graph_query(reason: &'static str) -> PolyglotError {
    PolyglotError::InvalidIdentifier {
        kind: "graph query",
        reason,
    }
}

use std::{collections::BTreeMap, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use futures::StreamExt;
use reqwest::{Client, Url, redirect::Policy};
use serde::{Deserialize, Serialize};

use super::{PolyglotError, TursoQueryLimit, TursoRow, TursoStatement, TursoValue};

const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

pub(super) struct HranaClient {
    client: Client,
    pipeline_url: Url,
    auth_token: String,
}

impl HranaClient {
    pub(super) fn new(endpoint: &str, auth_token: String) -> Result<Self, PolyglotError> {
        let pipeline_url = pipeline_url(endpoint)?;
        let client = Client::builder()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| PolyglotError::driver("Turso", error))?;
        Ok(Self {
            client,
            pipeline_url,
            auth_token,
        })
    }

    pub(super) async fn execute(&self, statement: TursoStatement) -> Result<u64, PolyglotError> {
        Ok(self
            .execute_statement(statement, false)
            .await?
            .affected_row_count)
    }

    pub(super) async fn query(
        &self,
        statement: TursoStatement,
        limit: TursoQueryLimit,
    ) -> Result<Vec<TursoRow>, PolyglotError> {
        let result = self.execute_statement(statement, true).await?;
        rows_from_result(result, limit)
    }

    pub(super) async fn transaction(
        &self,
        statements: Vec<TursoStatement>,
    ) -> Result<Vec<u64>, PolyglotError> {
        let statement_count = statements.len();
        let batch = transactional_batch(statements)?;
        let response = self
            .pipeline(vec![
                StreamRequest::Batch { batch },
                StreamRequest::Close {},
            ])
            .await?;
        let mut results = response.results.into_iter();
        let batch = expect_batch(stream_response(results.next(), "batch")?)?;
        expect_close(stream_response(results.next(), "close")?)?;
        if results.next().is_some() {
            return Err(protocol_error("pipeline returned extra results"));
        }
        if let Some(error) = batch.step_errors.into_iter().flatten().next() {
            return Err(server_error(error));
        }
        let affected = batch
            .step_results
            .into_iter()
            .skip(1)
            .take(statement_count)
            .map(|result| {
                result
                    .map(|result| result.affected_row_count)
                    .ok_or_else(|| protocol_error("transaction statement result is missing"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if affected.len() != statement_count {
            return Err(protocol_error("transaction returned too few results"));
        }
        Ok(affected)
    }

    async fn execute_statement(
        &self,
        statement: TursoStatement,
        want_rows: bool,
    ) -> Result<StatementResult, PolyglotError> {
        let response = self
            .pipeline(vec![
                StreamRequest::Execute {
                    stmt: WireStatement::new(statement, want_rows),
                },
                StreamRequest::Close {},
            ])
            .await?;
        let mut results = response.results.into_iter();
        let result = expect_execute(stream_response(results.next(), "execute")?)?;
        expect_close(stream_response(results.next(), "close")?)?;
        if results.next().is_some() {
            return Err(protocol_error("pipeline returned extra results"));
        }
        Ok(result)
    }

    async fn pipeline(
        &self,
        requests: Vec<StreamRequest>,
    ) -> Result<PipelineResponse, PolyglotError> {
        let mut request = self
            .client
            .post(self.pipeline_url.clone())
            .header(reqwest::header::ACCEPT, "application/json")
            .header("x-libsql-client-version", "rullst-orm-12");
        if !self.auth_token.is_empty() {
            request = request.bearer_auth(&self.auth_token);
        }
        let response = request
            .json(&PipelineRequest {
                baton: None,
                requests,
            })
            .send()
            .await
            .map_err(|error| PolyglotError::driver("Turso", error))?;
        if !response.status().is_success() {
            return Err(PolyglotError::Driver {
                backend: "Turso",
                message: format!("Hrana HTTP status {}", response.status()),
            });
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(response_too_large());
        }
        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| PolyglotError::driver("Turso", error))?;
            if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(response_too_large());
            }
            body.extend_from_slice(&chunk);
        }
        let response: PipelineResponse =
            serde_json::from_slice(&body).map_err(PolyglotError::serialization)?;
        if response.baton.is_some() {
            return Err(protocol_error("server kept a stream open after close"));
        }
        Ok(response)
    }
}

fn pipeline_url(endpoint: &str) -> Result<Url, PolyglotError> {
    let mut url = Url::parse(endpoint).map_err(|_| PolyglotError::InvalidConfiguration {
        backend: "Turso",
        reason: "endpoint is not a valid URL",
    })?;
    if url.scheme() == "libsql" {
        let suffix =
            url.as_str()
                .strip_prefix("libsql:")
                .ok_or(PolyglotError::InvalidConfiguration {
                    backend: "Turso",
                    reason: "libsql endpoint cannot be converted to HTTPS",
                })?;
        url = Url::parse(&format!("https:{suffix}")).map_err(|_| {
            PolyglotError::InvalidConfiguration {
                backend: "Turso",
                reason: "libsql endpoint cannot be converted to HTTPS",
            }
        })?;
    }
    let mut path = url
        .path_segments_mut()
        .map_err(|_| PolyglotError::InvalidConfiguration {
            backend: "Turso",
            reason: "endpoint cannot be used as an HTTP base URL",
        })?;
    path.pop_if_empty();
    path.push("v3").push("pipeline");
    drop(path);
    Ok(url)
}

fn transactional_batch(statements: Vec<TursoStatement>) -> Result<Batch, PolyglotError> {
    let mut steps = Vec::with_capacity(statements.len() + 3);
    steps.push(BatchStep::unconditional("BEGIN TRANSACTION"));
    for statement in statements {
        let previous = batch_step_index(&steps)?;
        steps.push(BatchStep {
            condition: Some(BatchCondition::Ok { step: previous }),
            stmt: WireStatement::new(statement, false),
        });
    }
    let previous = batch_step_index(&steps)?;
    steps.push(BatchStep {
        condition: Some(BatchCondition::Ok { step: previous }),
        stmt: WireStatement::literal("COMMIT"),
    });
    let commit = batch_step_index(&steps)?;
    steps.push(BatchStep {
        condition: Some(BatchCondition::Not {
            cond: Box::new(BatchCondition::Ok { step: commit }),
        }),
        stmt: WireStatement::literal("ROLLBACK"),
    });
    Ok(Batch { steps })
}

fn batch_step_index(steps: &[BatchStep]) -> Result<u32, PolyglotError> {
    let index = steps.len().checked_sub(1).ok_or_else(|| {
        protocol_error("transaction batch cannot reference an empty step sequence")
    })?;
    u32::try_from(index).map_err(|_| protocol_error("transaction batch has too many steps"))
}

fn rows_from_result(
    result: StatementResult,
    limit: TursoQueryLimit,
) -> Result<Vec<TursoRow>, PolyglotError> {
    let names = result
        .cols
        .into_iter()
        .enumerate()
        .map(|(index, column)| {
            column
                .name
                .filter(|name| !name.is_empty())
                .ok_or_else(|| protocol_error(format!("result column {index} has no name")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut unique = std::collections::BTreeSet::new();
    if names.iter().any(|name| !unique.insert(name.as_str())) {
        return Err(protocol_error("result contains duplicate column names"));
    }
    result
        .rows
        .into_iter()
        .take(limit.get() as usize)
        .map(|values| {
            if values.len() != names.len() {
                return Err(protocol_error("result row does not match its column count"));
            }
            let values = values
                .into_iter()
                .map(TursoValue::try_from)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(TursoRow {
                columns: names
                    .iter()
                    .cloned()
                    .zip(values)
                    .collect::<BTreeMap<_, _>>(),
            })
        })
        .collect()
}

#[derive(Serialize)]
struct PipelineRequest {
    baton: Option<String>,
    requests: Vec<StreamRequest>,
}

#[derive(Deserialize)]
struct PipelineResponse {
    baton: Option<String>,
    results: Vec<StreamResult>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamRequest {
    Execute { stmt: WireStatement },
    Batch { batch: Batch },
    Close {},
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamResult {
    Ok { response: StreamResponse },
    Error { error: ServerError },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamResponse {
    Execute { result: StatementResult },
    Batch { result: BatchResult },
    Close {},
}

#[derive(Serialize)]
struct WireStatement {
    sql: String,
    args: Vec<WireValue>,
    want_rows: bool,
}

impl WireStatement {
    fn new(statement: TursoStatement, want_rows: bool) -> Self {
        Self {
            sql: statement.sql,
            args: statement
                .parameters
                .into_iter()
                .map(WireValue::from)
                .collect(),
            want_rows,
        }
    }

    fn literal(sql: &str) -> Self {
        Self {
            sql: sql.to_owned(),
            args: Vec::new(),
            want_rows: false,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WireValue {
    Null,
    Integer { value: String },
    Float { value: f64 },
    Text { value: String },
    Blob { base64: String },
}

impl From<TursoValue> for WireValue {
    fn from(value: TursoValue) -> Self {
        match value {
            TursoValue::Null => Self::Null,
            TursoValue::Integer(value) => Self::Integer {
                value: value.to_string(),
            },
            TursoValue::Real(value) => Self::Float { value },
            TursoValue::Text(value) => Self::Text { value },
            TursoValue::Blob(value) => Self::Blob {
                base64: BASE64.encode(value),
            },
        }
    }
}

impl TryFrom<WireValue> for TursoValue {
    type Error = PolyglotError;

    fn try_from(value: WireValue) -> Result<Self, Self::Error> {
        match value {
            WireValue::Null => Ok(Self::Null),
            WireValue::Integer { value } => value
                .parse()
                .map(Self::Integer)
                .map_err(PolyglotError::serialization),
            WireValue::Float { value } => Ok(Self::Real(value)),
            WireValue::Text { value } => Ok(Self::Text(value)),
            WireValue::Blob { base64 } => BASE64
                .decode(base64)
                .map(Self::Blob)
                .map_err(PolyglotError::serialization),
        }
    }
}

#[derive(Serialize)]
struct Batch {
    steps: Vec<BatchStep>,
}

#[derive(Serialize)]
struct BatchStep {
    #[serde(skip_serializing_if = "Option::is_none")]
    condition: Option<BatchCondition>,
    stmt: WireStatement,
}

impl BatchStep {
    fn unconditional(sql: &str) -> Self {
        Self {
            condition: None,
            stmt: WireStatement::literal(sql),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BatchCondition {
    Ok { step: u32 },
    Not { cond: Box<BatchCondition> },
}

#[derive(Deserialize)]
struct BatchResult {
    step_results: Vec<Option<StatementResult>>,
    step_errors: Vec<Option<ServerError>>,
}

#[derive(Deserialize)]
struct StatementResult {
    #[serde(default)]
    cols: Vec<Column>,
    #[serde(default)]
    rows: Vec<Vec<WireValue>>,
    #[serde(default)]
    affected_row_count: u64,
}

#[derive(Deserialize)]
struct Column {
    name: Option<String>,
}

#[derive(Deserialize)]
struct ServerError {
    message: String,
    #[serde(default)]
    code: Option<String>,
}

fn stream_response(
    result: Option<StreamResult>,
    expected: &'static str,
) -> Result<StreamResponse, PolyglotError> {
    match result {
        Some(StreamResult::Ok { response }) => Ok(response),
        Some(StreamResult::Error { error }) => Err(server_error(error)),
        None => Err(protocol_error(format!(
            "missing {expected} pipeline result"
        ))),
    }
}

fn expect_execute(response: StreamResponse) -> Result<StatementResult, PolyglotError> {
    match response {
        StreamResponse::Execute { result } => Ok(result),
        _ => Err(protocol_error("expected execute response")),
    }
}

fn expect_batch(response: StreamResponse) -> Result<BatchResult, PolyglotError> {
    match response {
        StreamResponse::Batch { result } => Ok(result),
        _ => Err(protocol_error("expected batch response")),
    }
}

fn expect_close(response: StreamResponse) -> Result<(), PolyglotError> {
    match response {
        StreamResponse::Close {} => Ok(()),
        _ => Err(protocol_error("expected close response")),
    }
}

fn server_error(error: ServerError) -> PolyglotError {
    let code = error.code.filter(|code| !code.is_empty());
    PolyglotError::Driver {
        backend: "Turso",
        message: match code {
            Some(code) => format!("Hrana server error {code}: {}", error.message),
            None => format!("Hrana server error: {}", error.message),
        },
    }
}

fn protocol_error(message: impl Into<String>) -> PolyglotError {
    PolyglotError::Driver {
        backend: "Turso",
        message: format!("invalid Hrana response: {}", message.into()),
    }
}

fn response_too_large() -> PolyglotError {
    PolyglotError::ResponseTooLarge {
        backend: "Turso",
        limit_bytes: MAX_RESPONSE_BYTES,
    }
}

#[cfg(test)]
#[path = "hrana/tests.rs"]
mod tests;

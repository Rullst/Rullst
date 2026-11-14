//! An insert whose primary key the database generates (a text key's
//! `DEFAULT`), run in a transaction on one interactive Hrana stream and
//! committed only when the returned key is not NULL.

use super::{
    Batch, BatchCondition, BatchStep, HranaClient, PipelineResponse, PolyglotError, StreamRequest,
    StreamResult, TursoQueryLimit, TursoStatement, TursoValue, WireStatement, expect_batch,
    expect_close, expect_execute, protocol_error, rows_from_result, server_error, stream_response,
};

impl HranaClient {
    /// Runs `insert`, an `INSERT ... RETURNING <key_column>`, inside a
    /// transaction and commits it only when the returned key is not NULL.
    /// `Ok(None)`: the key was NULL and the insert was rolled back. Should the
    /// rollback itself not arrive, the server drops the uncommitted
    /// transaction with the stream.
    pub(crate) async fn insert_returning_key(
        &self,
        insert: TursoStatement,
        key_column: &str,
    ) -> Result<Option<TursoValue>, PolyglotError> {
        let opened = self
            .post(
                None,
                vec![StreamRequest::Batch {
                    batch: Batch {
                        steps: vec![
                            BatchStep::unconditional("BEGIN"),
                            BatchStep {
                                condition: Some(BatchCondition::Ok { step: 0 }),
                                stmt: WireStatement::new(insert, true),
                            },
                        ],
                    },
                }],
            )
            .await?;
        let Some(baton) = opened.baton else {
            return Err(protocol_error("server closed the insert's stream"));
        };
        let key = inserted_key(opened.results, key_column);
        let commit = matches!(key, Ok(Some(_)));
        let finish = if commit { "COMMIT" } else { "ROLLBACK" };
        let finished = self
            .closing(
                Some(baton),
                vec![
                    StreamRequest::Execute {
                        stmt: WireStatement::literal(finish),
                    },
                    StreamRequest::Close {},
                ],
            )
            .await;
        if commit {
            committed(finished?)?;
        }
        key
    }
}

/// The key the insert returned; `None` when it is NULL or absent.
fn inserted_key(
    results: Vec<StreamResult>,
    key_column: &str,
) -> Result<Option<TursoValue>, PolyglotError> {
    let batch = expect_batch(stream_response(results.into_iter().next(), "batch")?)?;
    if let Some(error) = batch.step_errors.into_iter().flatten().next() {
        return Err(server_error(error));
    }
    let inserted = batch
        .step_results
        .into_iter()
        .nth(1)
        .flatten()
        .ok_or_else(|| protocol_error("insert result is missing"))?;
    let row = rows_from_result(inserted, TursoQueryLimit::new(1)?)?
        .into_iter()
        .next();
    Ok(row
        .and_then(|row| row.into_columns().remove(key_column))
        .filter(|key| !matches!(key, TursoValue::Null)))
}

/// Checks that the server committed and closed the stream.
fn committed(response: PipelineResponse) -> Result<(), PolyglotError> {
    let mut results = response.results.into_iter();
    expect_execute(stream_response(results.next(), "commit")?)?;
    expect_close(stream_response(results.next(), "close")?)
}

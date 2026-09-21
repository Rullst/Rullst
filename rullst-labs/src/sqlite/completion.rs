use super::{
    exercises::bounded,
    grading::{ResultEvidence, grade},
    record::{JobState, JobView},
    store::SqliteLabs,
    transaction::Operation,
};
use crate::{
    Clock, ContentHash, ExecutionProfile, LabError as Error, Reference, Scope, SignedReceipt,
    Teardown, WorkerOutput,
};

impl<C: Clock> SqliteLabs<C> {
    /// Authenticated runner statement. Signature validation is against the pinned
    /// controller public key, never a key from the submitted receipt. Must not be
    /// wired to an unauthenticated application route.
    pub async fn complete(
        &self,
        scope: &Scope,
        id: &Reference,
        signed: &SignedReceipt,
    ) -> Result<JobView, Error> {
        let ExecutionProfile::LinuxExperimental { receipt_key, .. } = &self.config.profile else {
            return Err(Error::Unsupported);
        };
        signed.verify(receipt_key)?;
        if signed.receipt.teardown != Teardown::Confirmed {
            return Err(Error::Uncertain);
        }
        let digest = ContentHash::of(&serde_json::to_vec(signed).map_err(|_| Error::Protocol)?);
        let evidence = ResultEvidence::Experimental {
            receipt: digest,
            observations: signed.receipt.observation_digest.clone(),
        };
        self.finish(
            scope,
            id,
            &signed.receipt.output,
            evidence,
            Some((signed.receipt.started_at, signed.receipt.finished_at)),
        )
        .await
    }
    /// Deliberate protocol testing only. A simulation store produces Simulated,
    /// never Completed, and an experimental profile rejects this method.
    pub async fn complete_simulation(
        &self,
        scope: &Scope,
        id: &Reference,
        output: &WorkerOutput,
    ) -> Result<JobView, Error> {
        if self.config.profile != ExecutionProfile::Simulation {
            return Err(Error::Unsupported);
        }
        self.finish(scope, id, output, ResultEvidence::Simulation, None)
            .await
    }
    async fn finish(
        &self,
        scope: &Scope,
        id: &Reference,
        output: &WorkerOutput,
        evidence: ResultEvidence,
        timing: Option<(i64, i64)>,
    ) -> Result<JobView, Error> {
        output.validate()?;
        bounded(async {
            let mut tx = Operation::begin(self, None).await?;
            let (mut record, content) = self.load_job(&mut tx, scope, id).await?;
            // Exact receipt replay is an idempotent read, never a second award.
            if let Some(result) = &record.view.result {
                if result.evidence() == &evidence && !matches!(evidence, ResultEvidence::Simulation)
                {
                    tx.commit().await?;
                    return Ok(record.view);
                }
                return Err(Error::Conflict);
            }
            if record.view.state != JobState::Running || record.binding()? != output.binding {
                return Err(Error::Conflict);
            }
            let lease = record.lease.as_ref().ok_or(Error::Integrity)?;
            if lease.until <= tx.now || record.view.expires_at <= tx.now {
                return Err(Error::Expired);
            }
            tx.narrow_deadline(lease.until.min(record.view.expires_at))?;
            if timing.is_some_and(|(started, finished)| {
                started < record.view.updated_at || finished > tx.now || finished >= lease.until
            }) {
                return Err(Error::Protocol);
            }
            let (exercise, enabled) = self
                .load_exercise(
                    &mut tx,
                    scope,
                    &record.view.exercise.id,
                    &record.view.exercise.revision,
                )
                .await?;
            if !enabled {
                return Err(Error::Denied);
            }
            if exercise.digest()? != record.view.exercise_digest {
                return Err(Error::Integrity);
            }
            let content = content.ok_or(Error::Integrity)?;
            self.payload(&record, &content)?;
            let result = grade(&exercise, &output.outcome, evidence)?;
            let revision = record.view.revision;
            record.view.state = if matches!(result.evidence(), ResultEvidence::Simulation) {
                JobState::Simulated
            } else if matches!(result, super::grading::JobResult::Graded { .. }) {
                JobState::Completed
            } else {
                JobState::Failed
            };
            record.view.result = Some(result);
            record.lease = None;
            record.next(tx.now)?;
            self.save_job(&mut tx, &record, None, revision).await?;
            tx.commit().await?;
            Ok(record.view)
        })
        .await
    }
}

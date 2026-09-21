use super::{
    exercises::bounded,
    record::{JobState, JobView},
    store::{SqliteLabs, storage},
    transaction::Operation,
};
use crate::{
    AttemptBinding, Clock, ExecutionFailure, ExecutionProfile, LabError as Error, Reference, Scope,
    SignedReceipt, Teardown, WorkerOutcome,
};

#[derive(Debug, Clone)]
pub struct CleanupJob {
    pub scope: Scope,
    pub id: Reference,
    pub binding: AttemptBinding,
    pub lease_until: i64,
}
impl<C: Clock> SqliteLabs<C> {
    /// Fences a controller-owned failed attempt immediately, before teardown.
    /// An old nonce cannot fence a newer attempt. This is a dedicated job-plane
    /// operation and must never be exposed as a student capability.
    pub async fn abandon_attempt(&self, job: &super::LeasedJob) -> Result<CleanupJob, Error> {
        bounded(async {
            let mut tx = Operation::begin(self, None).await?;
            let (mut record, content) = self.load_job(&mut tx, job.scope(), job.id()).await?;
            if record.binding()? != *job.input().binding() {
                return Err(Error::Conflict);
            }
            let revision = record.view.revision;
            if record.view.state == JobState::Running {
                record.view.state = if record.view.expires_at <= tx.now {
                    JobState::Expired
                } else {
                    JobState::Uncertain
                };
                record.next(tx.now)?;
                self.save_job(&mut tx, &record, content.as_deref(), revision)
                    .await?;
            }
            let lease = record.lease.as_ref().ok_or(Error::Integrity)?;
            let cleanup = CleanupJob {
                scope: job.scope().clone(),
                id: job.id().clone(),
                binding: record.binding()?,
                lease_until: lease.until,
            };
            tx.commit().await?;
            Ok(cleanup)
        })
        .await
    }
    /// Dedicated controller only. Fences expired leases before asking the runner
    /// to kill/reap its owned cgroup and remove the disposable workspace. Finding
    /// an item here does not prove cleanup and never itself permits a retry.
    pub async fn cleanup_candidates(&self, limit: u32) -> Result<Vec<CleanupJob>, Error> {
        if !(1..=32).contains(&limit) {
            return Err(Error::InvalidInput);
        }
        bounded(async {
            let mut tx=Operation::begin(self,None).await?;
            let keys:Vec<(String,String,String)>=sqlx::query_as("SELECT substr(tenant,1,97),substr(course,1,97),substr(id,1,97) FROM labs_jobs WHERE lease_until>0 AND (state<>'Running' OR lease_until<=? OR expires_at<=?) ORDER BY lease_until,tenant,course,id LIMIT ?")
                .bind(tx.now).bind(tx.now).bind(limit).fetch_all(&mut *tx.tx).await.map_err(storage)?;
            let mut jobs=Vec::with_capacity(keys.len());
            for (tenant,course,id) in keys {
                let scope=Scope::new(tenant,course).map_err(|_|Error::Integrity)?;
                let id=Reference::new(id).map_err(|_|Error::Integrity)?;
                let (mut record,content)=self.load_job(&mut tx,&scope,&id).await?;
                let revision=record.view.revision;
                if record.view.state==JobState::Running {
                    record.view.state=if record.view.expires_at<=tx.now {JobState::Expired} else {JobState::Uncertain};
                    record.next(tx.now)?;
                    self.save_job(&mut tx,&record,content.as_deref(),revision).await?;
                }
                let lease=record.lease.as_ref().ok_or(Error::Integrity)?;
                jobs.push(CleanupJob {scope,id,binding:record.binding()?,lease_until:lease.until});
            }
            tx.commit().await?;Ok(jobs)
        }).await
    }
    /// Only a controller-signed worker-loss receipt with confirmed teardown may
    /// reconcile a real abandoned attempt. It never awards a grade. Retrying a
    /// pure function is explicit, capped at two attempts and requires a new nonce.
    pub async fn reconcile_cleanup(
        &self,
        scope: &Scope,
        id: &Reference,
        signed: &SignedReceipt,
        retry: bool,
    ) -> Result<JobView, Error> {
        let ExecutionProfile::LinuxExperimental { receipt_key, .. } = &self.config.profile else {
            return Err(Error::Unsupported);
        };
        signed.verify(receipt_key)?;
        if signed.receipt.teardown != Teardown::Confirmed
            || signed.receipt.output.outcome
                != WorkerOutcome::Rejected(ExecutionFailure::WorkerLost)
        {
            return Err(Error::Uncertain);
        }
        self.finish_cleanup(scope, id, &signed.receipt.output.binding, retry)
            .await
    }
    pub async fn reconcile_simulation_cleanup(
        &self,
        job: &CleanupJob,
        retry: bool,
    ) -> Result<JobView, Error> {
        if self.config.profile != ExecutionProfile::Simulation {
            return Err(Error::Unsupported);
        }
        self.finish_cleanup(&job.scope, &job.id, &job.binding, retry)
            .await
    }
    async fn finish_cleanup(
        &self,
        scope: &Scope,
        id: &Reference,
        binding: &AttemptBinding,
        retry: bool,
    ) -> Result<JobView, Error> {
        bounded(async {
            let mut tx = Operation::begin(self, None).await?;
            let (mut record, mut content) = self.load_job(&mut tx, scope, id).await?;
            if !matches!(
                record.view.state,
                JobState::Cancelled | JobState::Expired | JobState::Uncertain
            ) || record.binding()? != *binding
            {
                return Err(Error::Conflict);
            }
            let (exercise, enabled) = self
                .load_exercise(
                    &mut tx,
                    scope,
                    &record.view.exercise.id,
                    &record.view.exercise.revision,
                )
                .await?;
            let revision = record.view.revision;
            if record.view.state == JobState::Uncertain {
                record.view.state = if record.view.expires_at.saturating_sub(tx.now)
                    <= i64::from(exercise.limits().wall_seconds()) + 5
                {
                    JobState::Expired
                } else if retry && enabled && record.attempts < 2 {
                    JobState::Queued
                } else {
                    JobState::Cancelled
                };
            }
            if record.view.state != JobState::Queued {
                content = None;
            }
            record.lease = None;
            record.next(tx.now)?;
            self.save_job(&mut tx, &record, content.as_deref(), revision)
                .await?;
            tx.commit().await?;
            Ok(record.view)
        })
        .await
    }
}

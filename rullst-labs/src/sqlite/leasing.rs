use super::{
    exercises::bounded,
    record::{JobState, Lease, Record},
    store::{SqliteLabs, storage},
    transaction::Operation,
};
use crate::{AttemptBinding, Clock, LabError as Error, Reference, Scope, WorkerInput};
use ring::rand::{SecureRandom, SystemRandom};

/// Runner-controller capability obtained from the dedicated job plane, not from
/// a student request. Source is sent only after the OS boundary is enforced.
#[derive(Debug)]
pub struct LeasedJob {
    pub(super) scope: Scope,
    pub(super) id: Reference,
    pub(super) revision: i64,
    pub(super) until: i64,
    pub(super) input: WorkerInput,
}
impl LeasedJob {
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn id(&self) -> &Reference {
        &self.id
    }
    pub fn revision(&self) -> i64 {
        self.revision
    }
    pub fn expires_at(&self) -> i64 {
        self.until
    }
    pub fn input(&self) -> &WorkerInput {
        &self.input
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseStatus {
    Active,
    Stop,
}
impl<C: Clock> SqliteLabs<C> {
    /// Dedicated controller only. Claims at most one job, skipping up to 32
    /// expired/withdrawn items. The controller must reconcile abandoned groups
    /// on startup and monitor cancellation/deadline while executing this lease.
    pub async fn claim_next(&self) -> Result<Option<LeasedJob>, Error> {
        bounded(async {
            let mut tx=Operation::begin(self,None).await?;
            for _ in 0..32 {
                let key:Option<(String,String,String)>=sqlx::query_as("SELECT substr(tenant,1,97),substr(course,1,97),substr(id,1,97) FROM labs_jobs WHERE state='Queued' ORDER BY expires_at,tenant,course,id LIMIT 1").fetch_optional(&mut *tx.tx).await.map_err(storage)?;
                let Some((tenant,course,id))=key else { tx.commit().await?; return Ok(None); };
                let scope=Scope::new(tenant,course).map_err(|_|Error::Integrity)?;
                let id=Reference::new(id).map_err(|_|Error::Integrity)?;
                let (mut record,content)=self.load_job(&mut tx,&scope,&id).await?;
                let revision=record.view.revision;
                let (exercise,enabled)=self.load_exercise(&mut tx,&scope,&record.view.exercise.id,&record.view.exercise.revision).await?;
                if exercise.digest()?!=record.view.exercise_digest { return Err(Error::Integrity); }
                let needed=i64::from(exercise.limits().wall_seconds())+5;
                if !enabled || record.view.expires_at.saturating_sub(tx.now)<=needed || record.attempts>=2 {
                    record.view.state=if enabled {JobState::Expired} else {JobState::Cancelled};
                    record.next(tx.now)?;
                    self.save_job(&mut tx,&record,None,revision).await?;
                    continue;
                }
                let content=content.ok_or(Error::Integrity)?;
                let payload=self.payload(&record,&content)?;
                let mut nonce=[0u8;24]; SystemRandom::new().fill(&mut nonce).map_err(|_|Error::Storage)?;
                let until=tx.now.checked_add(i64::from(exercise.limits().wall_seconds())+15).ok_or(Error::Clock)?.min(record.view.expires_at);
                record.view.state=JobState::Running;
                record.attempts=record.attempts.checked_add(1).ok_or(Error::Capacity)?;
                record.lease=Some(Lease {nonce:Reference::new(hex::encode(nonce))?,until,revision:revision.checked_add(1).ok_or(Error::Capacity)?});
                record.next(tx.now)?;
                let input=WorkerInput::new(record.binding()?,payload.source,exercise.grader_cases().iter().map(|c|c.input).collect(),exercise.limits().clone())?;
                self.save_job(&mut tx,&record,Some(&content),revision).await?;
                let leased=LeasedJob {scope,id,revision:record.view.revision,until,input};
                tx.narrow_deadline(until)?;tx.commit().await?;return Ok(Some(leased));
            }
            tx.commit().await?; Ok(None)
        }).await
    }
    /// A read error is a stop signal for the controller. Active never extends
    /// the durable lease or authorizes execution after the local wall deadline.
    pub async fn lease_status(&self, job: &LeasedJob) -> Result<LeaseStatus, Error> {
        bounded(async {
            let mut tx = Operation::begin(self, None).await?;
            let (record, _) = self.load_job(&mut tx, &job.scope, &job.id).await?;
            let (_, enabled) = self
                .load_exercise(
                    &mut tx,
                    &job.scope,
                    &record.view.exercise.id,
                    &record.view.exercise.revision,
                )
                .await?;
            let active = enabled
                && record.view.state == JobState::Running
                && record.view.revision == job.revision
                && record
                    .lease
                    .as_ref()
                    .is_some_and(|l| l.until > tx.now && l.until == job.until)
                && record.binding()? == *job.input.binding();
            tx.commit().await?;
            Ok(if active {
                LeaseStatus::Active
            } else {
                LeaseStatus::Stop
            })
        })
        .await
    }
}
impl Record {
    pub(super) fn binding(&self) -> Result<AttemptBinding, Error> {
        let lease = self.lease.as_ref().ok_or(Error::Conflict)?;
        Ok(AttemptBinding {
            request: self.request_digest.clone(),
            profile: self.profile_digest.clone(),
            source: self.view.source_digest.clone(),
            nonce: lease.nonce.clone(),
        })
    }
}

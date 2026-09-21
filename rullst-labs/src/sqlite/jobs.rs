use super::{
    exercises::bounded,
    record::{JobState, JobView, Payload, Record},
    store::{SqliteLabs, storage},
    transaction::Operation,
};
use crate::{
    Action, Authorization, Clock, ContentHash, LabError as Error, Reference, Scope, Submission,
};

impl<C: Clock> SqliteLabs<C> {
    /// Durable idempotent submission. Only a registered, enabled instructor
    /// revision can supply the grader; scope/learner come from host authorization.
    pub async fn submit<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        submission: Submission,
    ) -> Result<JobView, Error> {
        bounded(async {
            let permission=self.permit(auth,actor,scope,Action::Submit).await?;
            let mut tx=Operation::begin(self,Some(permission.expires_at())).await?;
            let (exercise,enabled)=self.load_exercise(&mut tx,scope,&submission.exercise.id,&submission.exercise.revision).await?;
            if !enabled { return Err(Error::Denied); }
            let source_digest=submission.source.digest();
            let exercise_digest=exercise.digest()?;
            let profile_digest=self.config.profile.digest()?;
            let request_digest=ContentHash::of(&serde_json::to_vec(&(crate::PROTOCOL_VERSION,&profile_digest,scope,actor,&submission.id,&exercise_digest,&source_digest,submission.ttl_seconds)).map_err(|_|Error::InvalidInput)?);
            match self.load_job(&mut tx,scope,&submission.id).await {
                Ok((record,_))=>{
                    if record.view.learner!=*actor || record.request_digest!=request_digest { return Err(Error::Conflict); }
                    tx.commit().await?;return Ok(record.view);
                }
                Err(Error::NotFound)=>(),
                Err(error)=>return Err(error),
            }
            let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM labs_jobs").fetch_one(&mut *tx.tx).await.map_err(storage)?;
            if count>=i64::from(self.config.max_jobs) { return Err(Error::Capacity); }
            let expires_at=tx.now.checked_add(i64::from(submission.ttl_seconds)).ok_or(Error::Clock)?.min(permission.expires_at());
            let record=Record {
                view:JobView { id:submission.id,scope:scope.clone(),learner:actor.clone(),exercise:submission.exercise,source_digest,exercise_digest,state:JobState::Queued,revision:1,created_at:tx.now,updated_at:tx.now,expires_at,cleanup_pending:false,result:None },
                request_digest,profile_digest,lease:None,attempts:0,
            };
            record.validate(true)?;
            let payload=Payload { exercise,source:submission.source };
            let plaintext=zeroize::Zeroizing::new(serde_json::to_vec(&payload).map_err(|_|Error::InvalidInput)?);
            let content=self.key.seal(&self.job_aad(&record)?,&plaintext)?;
            let body=self.seal_record(&record)?;
            sqlx::query("INSERT INTO labs_jobs (tenant,course,id,learner,state,revision,expires_at,lease_until,body,content) VALUES (?,?,?,?,'Queued',1,?,0,?,?)")
                .bind(scope.tenant.as_str()).bind(scope.course.as_str()).bind(record.view.id.as_str()).bind(actor.as_str()).bind(expires_at).bind(body).bind(content)
                .execute(&mut *tx.tx).await.map_err(storage)?;
            tx.commit().await?;Ok(record.view)
        }).await
    }
    pub async fn get_job<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
    ) -> Result<JobView, Error> {
        bounded(async {
            // Check current course access before any lookup; the ownership check
            // below can only narrow permission to own jobs or explicit management.
            let permission = self
                .permit(auth, actor, scope, Action::AccessCourse)
                .await?;
            let mut tx = Operation::begin(self, Some(permission.expires_at())).await?;
            let (record, _) = self.load_job(&mut tx, scope, id).await?;
            tx.commit().await?;
            let action = if record.view.learner == *actor {
                Action::ReadOwn
            } else {
                Action::ManageJobs
            };
            self.permit(auth, actor, scope, action).await?;
            Ok(record.view)
        })
        .await
    }
    /// Records cancellation before worker teardown. A running lease remains
    /// visible as cleanup_pending and must be reconciled by the separate runner.
    pub async fn cancel<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        revision: i64,
    ) -> Result<JobView, Error> {
        bounded(async {
            let permission = self
                .permit(auth, actor, scope, Action::AccessCourse)
                .await?;
            let mut tx = Operation::begin(self, Some(permission.expires_at())).await?;
            let (mut record, mut content) = self.load_job(&mut tx, scope, id).await?;
            let action = if record.view.learner == *actor {
                Action::CancelOwn
            } else {
                Action::ManageJobs
            };
            tx.commit().await?;
            let permission = self.permit(auth, actor, scope, action).await?;
            if revision <= 0 || record.view.revision != revision {
                return Err(Error::Conflict);
            }
            if !matches!(record.view.state, JobState::Queued | JobState::Running) {
                return Err(Error::Conflict);
            }
            let mut tx = Operation::begin(self, Some(permission.expires_at())).await?;
            record.view.state = JobState::Cancelled;
            if record.lease.is_none() {
                content = None;
            }
            record.next(tx.now)?;
            self.save_job(&mut tx, &record, content.as_deref(), revision)
                .await?;
            tx.commit().await?;
            Ok(record.view)
        })
        .await
    }
    pub(super) async fn load_job(
        &self,
        tx: &mut Operation<'_, C>,
        scope: &Scope,
        id: &Reference,
    ) -> Result<(Record, Option<Vec<u8>>), Error> {
        type StoredJob = (String, String, i64, i64, i64, Vec<u8>, Option<Vec<u8>>);
        let row:Option<StoredJob>=sqlx::query_as("SELECT substr(learner,1,97),substr(state,1,33),revision,expires_at,lease_until,substr(body,1,32797),substr(content,1,262173) FROM labs_jobs WHERE tenant=? AND course=? AND id=?")
            .bind(scope.tenant.as_str()).bind(scope.course.as_str()).bind(id.as_str()).fetch_optional(&mut *tx.tx).await.map_err(storage)?;
        let (learner, state, revision, expires_at, lease_until, body, content) =
            row.ok_or(Error::NotFound)?;
        if body.len() > 32796 {
            return Err(Error::Integrity);
        }
        let plaintext = self.key.open(&self.record_aad(scope, id)?, &body)?;
        let record: Record = serde_json::from_slice(&plaintext).map_err(|_| Error::Integrity)?;
        record.validate(content.is_some())?;
        let v = &record.view;
        if v.scope != *scope
            || v.id != *id
            || v.learner.as_str() != learner
            || v.state.name() != state
            || v.revision != revision
            || v.expires_at != expires_at
            || record.lease.as_ref().map_or(0, |lease| lease.until) != lease_until
            || record.profile_digest != self.config.profile.digest()?
        {
            return Err(Error::Integrity);
        }
        if let Some(content) = &content {
            self.payload(&record, content)?;
        }
        Ok((record, content))
    }
    pub(super) fn payload(&self, record: &Record, content: &[u8]) -> Result<Payload, Error> {
        let plaintext = self.key.open(&self.job_aad(record)?, content)?;
        let payload: Payload = serde_json::from_slice(&plaintext).map_err(|_| Error::Integrity)?;
        let v = &record.view;
        if payload.source.digest() != v.source_digest
            || payload.exercise.digest()? != v.exercise_digest
            || payload.exercise.scope() != &v.scope
            || payload.exercise.id() != &v.exercise.id
            || payload.exercise.revision() != &v.exercise.revision
        {
            return Err(Error::Integrity);
        }
        Ok(payload)
    }
    pub(super) async fn save_job(
        &self,
        tx: &mut Operation<'_, C>,
        record: &Record,
        content: Option<&[u8]>,
        revision: i64,
    ) -> Result<(), Error> {
        record.validate(content.is_some())?;
        let body = self.seal_record(record)?;
        let v = &record.view;
        let result=sqlx::query("UPDATE labs_jobs SET state=?,revision=?,expires_at=?,lease_until=?,body=?,content=? WHERE tenant=? AND course=? AND id=? AND revision=?")
            .bind(v.state.name()).bind(v.revision).bind(v.expires_at).bind(record.lease.as_ref().map_or(0,|lease|lease.until)).bind(body).bind(content)
            .bind(v.scope.tenant.as_str()).bind(v.scope.course.as_str()).bind(v.id.as_str()).bind(revision).execute(&mut *tx.tx).await.map_err(storage)?;
        if result.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        Ok(())
    }
    fn record_aad(&self, scope: &Scope, id: &Reference) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(&(
            "RullstLabsRecord-v1",
            &self.config.namespace,
            self.config.profile.digest()?,
            scope,
            id,
        ))
        .map_err(|_| Error::Configuration)
    }
    fn seal_record(&self, record: &Record) -> Result<Vec<u8>, Error> {
        let body =
            zeroize::Zeroizing::new(serde_json::to_vec(record).map_err(|_| Error::Integrity)?);
        if body.len() > 32768 {
            return Err(Error::Capacity);
        }
        self.key.seal(
            &self.record_aad(&record.view.scope, &record.view.id)?,
            &body,
        )
    }
    fn job_aad(&self, record: &Record) -> Result<Vec<u8>, Error> {
        let v = &record.view;
        serde_json::to_vec(&(
            "RullstLabsJob-v1",
            &self.config.namespace,
            &record.profile_digest,
            &v.scope,
            &v.id,
            &v.learner,
            v.created_at,
            v.expires_at,
            &v.source_digest,
            &v.exercise_digest,
        ))
        .map_err(|_| Error::Configuration)
    }
}

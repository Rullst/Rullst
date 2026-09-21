use super::{
    exercises::bounded,
    record::JobState,
    store::{SqliteLabs, storage},
    transaction::Operation,
};
use crate::{Action, Authorization, Clock, LabError as Error, Reference, Scope};

impl<C: Clock> SqliteLabs<C> {
    /// Expires up to `limit` queued jobs in this course and clears their source.
    /// Hosts may schedule this without a runner. Leased work remains the
    /// controller's responsibility: expiry is never evidence of OS teardown.
    pub async fn expire_queued<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        limit: u32,
    ) -> Result<u32, Error> {
        if !(1..=100).contains(&limit) {
            return Err(Error::InvalidInput);
        }
        bounded(async {
            let permission = self.permit(auth, actor, scope, Action::ManageJobs).await?;
            let mut tx = Operation::begin(self, Some(permission.expires_at())).await?;
            let ids: Vec<String> = sqlx::query_scalar("SELECT substr(id,1,97) FROM labs_jobs WHERE tenant=? AND course=? AND state='Queued' AND expires_at<=? ORDER BY expires_at,id LIMIT ?")
                .bind(scope.tenant.as_str()).bind(scope.course.as_str()).bind(tx.now).bind(limit)
                .fetch_all(&mut *tx.tx).await.map_err(storage)?;
            let count = u32::try_from(ids.len()).map_err(|_| Error::Integrity)?;
            for id in ids {
                let id = Reference::new(id).map_err(|_| Error::Integrity)?;
                let (mut record, _) = self.load_job(&mut tx, scope, &id).await?;
                if record.view.state != JobState::Queued || record.view.expires_at > tx.now || record.lease.is_some() {
                    return Err(Error::Integrity);
                }
                let revision = record.view.revision;
                record.view.state = JobState::Expired;
                record.next(tx.now)?;
                self.save_job(&mut tx, &record, None, revision).await?;
            }
            tx.commit().await?;
            Ok(count)
        }).await
    }

    /// Permanently removes a withdrawn grader only after all referencing jobs
    /// have been purged. Do not reuse its revision ID after removal. This checks
    /// authenticated records rather than trusting unencrypted identity indexes.
    pub async fn remove_exercise<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        exercise: &crate::ExerciseRef,
    ) -> Result<(), Error> {
        bounded(async {
            let permission = self.permit(auth, actor, scope, Action::ManageExercises).await?;
            let mut tx = Operation::begin(self, Some(permission.expires_at())).await?;
            let (_, enabled) = self.load_exercise(&mut tx, scope, &exercise.id, &exercise.revision).await?;
            if enabled { return Err(Error::Conflict); }
            let ids: Vec<String> = sqlx::query_scalar("SELECT substr(id,1,97) FROM labs_jobs WHERE tenant=? AND course=? ORDER BY id LIMIT 1001")
                .bind(scope.tenant.as_str()).bind(scope.course.as_str()).fetch_all(&mut *tx.tx).await.map_err(storage)?;
            if ids.len() > self.config.max_jobs as usize { return Err(Error::Integrity); }
            for id in ids {
                let id = Reference::new(id).map_err(|_| Error::Integrity)?;
                let (record, _) = self.load_job(&mut tx, scope, &id).await?;
                if &record.view.exercise == exercise { return Err(Error::Conflict); }
            }
            let result = sqlx::query("DELETE FROM labs_exercises WHERE tenant=? AND course=? AND id=? AND revision=? AND enabled=0")
                .bind(scope.tenant.as_str()).bind(scope.course.as_str()).bind(exercise.id.as_str()).bind(exercise.revision.as_str())
                .execute(&mut *tx.tx).await.map_err(storage)?;
            if result.rows_affected() != 1 { return Err(Error::Conflict); }
            tx.commit().await
        }).await
    }

    /// Removes terminal status/idempotency records only after at least 24 hours
    /// and confirmed teardown. Source/grader snapshots are already erased when
    /// a job reaches a reconciled terminal state. Idempotency is scoped to this
    /// retention window; hosts should generate new random IDs after removal.
    /// Backups, WAL pages and physical erasure remain operator responsibilities.
    pub async fn purge_terminal<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        min_age_seconds: u32,
        limit: u32,
    ) -> Result<u32, Error> {
        if !(86_400..=31_536_000).contains(&min_age_seconds) || !(1..=100).contains(&limit) {
            return Err(Error::InvalidInput);
        }
        bounded(async {
            let permission=self.permit(auth,actor,scope,Action::ManageJobs).await?;
            let mut tx=Operation::begin(self,Some(permission.expires_at())).await?;
            // At most 1000 configured rows; bounded selection is followed by
            // authenticated record validation, never blind trust in SQL indexes.
            let ids:Vec<String>=sqlx::query_scalar("SELECT substr(id,1,97) FROM labs_jobs WHERE tenant=? AND course=? AND lease_until=0 AND state NOT IN ('Queued','Running') ORDER BY expires_at,id LIMIT 1000")
                .bind(scope.tenant.as_str()).bind(scope.course.as_str()).fetch_all(&mut *tx.tx).await.map_err(storage)?;
            let mut removed=0;
            for id in ids {
                let id=Reference::new(id).map_err(|_|Error::Integrity)?;
                let (record,content)=self.load_job(&mut tx,scope,&id).await?;
                if matches!(record.view.state,JobState::Queued|JobState::Running|JobState::Uncertain) || record.lease.is_some() || content.is_some() { return Err(Error::Integrity); }
                if tx.now.saturating_sub(record.view.updated_at)<i64::from(min_age_seconds) {continue;}
                let result=sqlx::query("DELETE FROM labs_jobs WHERE tenant=? AND course=? AND id=? AND revision=?")
                    .bind(scope.tenant.as_str()).bind(scope.course.as_str()).bind(id.as_str()).bind(record.view.revision).execute(&mut *tx.tx).await.map_err(storage)?;
                if result.rows_affected()!=1 {return Err(Error::Conflict);}
                removed+=1;if removed==limit {break;}
            }
            tx.commit().await?;Ok(removed)
        }).await
    }
}

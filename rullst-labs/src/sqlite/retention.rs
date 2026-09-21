use super::{
    exercises::bounded,
    record::JobState,
    store::{SqliteLabs, storage},
    transaction::Operation,
};
use crate::{Action, Authorization, Clock, LabError as Error, Reference, Scope};

impl<C: Clock> SqliteLabs<C> {
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

use super::{
    store::{SqliteLabs, storage},
    transaction::Operation,
};
use crate::{
    Action, Authorization, Clock, ContentHash, Exercise, LabError as Error, Permission, Reference,
    Scope, authorization::checked_time,
};
use serde::{Deserialize, Serialize};
use std::{future::Future, time::Duration};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisteredExercise {
    exercise: Exercise,
    enabled: bool,
}

impl<C: Clock> SqliteLabs<C> {
    pub(super) async fn permit<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        action: Action,
    ) -> Result<Permission, Error> {
        let permission = auth.check(actor, scope, action).await?;
        if permission.expires_at() <= checked_time(self.clock.now()?)? {
            return Err(Error::Denied);
        }
        Ok(permission)
    }
    /// Registers an immutable, enabled instructor-owned revision. Retrying an
    /// existing identical revision cannot silently re-enable a withdrawn one.
    pub async fn register_exercise<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        exercise: &Exercise,
    ) -> Result<(), Error> {
        bounded(async {
            let permission=self.permit(auth,actor,exercise.scope(),Action::ManageExercises).await?;
            let digest=exercise.digest()?;
            let mut tx=Operation::begin(self,Some(permission.expires_at())).await?;
            match self.load_exercise(&mut tx,exercise.scope(),exercise.id(),exercise.revision()).await {
                Ok((existing,_))=>{
                    if existing.digest()? != digest { return Err(Error::Conflict); }
                }
                Err(Error::NotFound)=>{
                    let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM labs_exercises").fetch_one(&mut *tx.tx).await.map_err(storage)?;
                    if count>=i64::from(self.config.max_exercises) { return Err(Error::Capacity); }
                    let plaintext=zeroize::Zeroizing::new(serde_json::to_vec(&RegisteredExercise { exercise:exercise.clone(),enabled:true }).map_err(|_|Error::InvalidInput)?);
                    let content=self.key.seal(&self.exercise_aad(exercise.scope(),exercise.id(),exercise.revision())?, &plaintext)?;
                    sqlx::query("INSERT INTO labs_exercises (tenant,course,id,revision,digest,enabled,content) VALUES (?,?,?,?,?,1,?)")
                        .bind(exercise.scope().tenant.as_str()).bind(exercise.scope().course.as_str()).bind(exercise.id().as_str()).bind(exercise.revision().as_str()).bind(digest.as_str()).bind(content)
                        .execute(&mut *tx.tx).await.map_err(storage)?;
                }
                Err(error)=>return Err(error),
            }
            tx.commit().await
        }).await
    }
    /// Management-only access to the secret grader. Student routes must expose
    /// their own public exercise presentation, never this value.
    pub async fn get_exercise<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        revision: &Reference,
    ) -> Result<(Exercise, bool), Error> {
        bounded(async {
            let permission = self
                .permit(auth, actor, scope, Action::ManageExercises)
                .await?;
            let mut tx = Operation::begin(self, Some(permission.expires_at())).await?;
            let result = self.load_exercise(&mut tx, scope, id, revision).await?;
            tx.commit().await?;
            self.permit(auth, actor, scope, Action::ManageExercises)
                .await?;
            Ok(result)
        })
        .await
    }
    /// Explicit publication control. A runner must recheck this state before
    /// accepting a grade; registering a prior revision never overrides it.
    pub async fn set_exercise_enabled<A: Authorization>(
        &self,
        auth: &A,
        actor: &Reference,
        scope: &Scope,
        id: &Reference,
        revision: &Reference,
        enabled: bool,
    ) -> Result<(), Error> {
        bounded(async {
            let permission=self.permit(auth,actor,scope,Action::ManageExercises).await?;
            let mut tx=Operation::begin(self,Some(permission.expires_at())).await?;
            let (exercise,_)=self.load_exercise(&mut tx,scope,id,revision).await?;
            let plaintext=zeroize::Zeroizing::new(serde_json::to_vec(&RegisteredExercise {exercise,enabled}).map_err(|_|Error::Integrity)?);
            let content=self.key.seal(&self.exercise_aad(scope,id,revision)?,&plaintext)?;
            let result=sqlx::query("UPDATE labs_exercises SET enabled=?,content=? WHERE tenant=? AND course=? AND id=? AND revision=?")
                .bind(enabled).bind(content).bind(scope.tenant.as_str()).bind(scope.course.as_str()).bind(id.as_str()).bind(revision.as_str()).execute(&mut *tx.tx).await.map_err(storage)?;
            if result.rows_affected()!=1 { return Err(Error::Conflict); }
            tx.commit().await
        }).await
    }
    pub(super) async fn load_exercise(
        &self,
        tx: &mut Operation<'_, C>,
        scope: &Scope,
        id: &Reference,
        revision: &Reference,
    ) -> Result<(Exercise, bool), Error> {
        let row:Option<(String,i64,Vec<u8>)>=sqlx::query_as("SELECT substr(digest,1,65),enabled,substr(content,1,262173) FROM labs_exercises WHERE tenant=? AND course=? AND id=? AND revision=?")
            .bind(scope.tenant.as_str()).bind(scope.course.as_str()).bind(id.as_str()).bind(revision.as_str()).fetch_optional(&mut *tx.tx).await.map_err(storage)?;
        let (digest, enabled, sealed) = row.ok_or(Error::NotFound)?;
        let plaintext = self
            .key
            .open(&self.exercise_aad(scope, id, revision)?, &sealed)?;
        let registered: RegisteredExercise =
            serde_json::from_slice(&plaintext).map_err(|_| Error::Integrity)?;
        let exercise = registered.exercise;
        if exercise.scope() != scope
            || exercise.id() != id
            || exercise.revision() != revision
            || exercise.digest()? != ContentHash::new(digest).map_err(|_| Error::Integrity)?
            || !(0..=1).contains(&enabled)
            || registered.enabled != (enabled == 1)
        {
            return Err(Error::Integrity);
        }
        Ok((exercise, enabled == 1))
    }
    fn exercise_aad(
        &self,
        scope: &Scope,
        id: &Reference,
        revision: &Reference,
    ) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(&(
            "RullstLabsExercise-v1",
            &self.config.namespace,
            self.config.profile.digest()?,
            scope,
            id,
            revision,
        ))
        .map_err(|_| Error::Configuration)
    }
}
pub(super) async fn bounded<T>(work: impl Future<Output = Result<T, Error>>) -> Result<T, Error> {
    tokio::time::timeout(Duration::from_secs(10), work)
        .await
        .map_err(|_| Error::Uncertain)?
}

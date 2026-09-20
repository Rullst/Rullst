use super::{SqliteSupervision, storage, transaction::Operation};
use crate::{
    AuthorityAction, Clock, Context, OpaqueId, Operator, Revision, Scope,
    SupervisionError as Error,
    parental::{AccessDecision, CoursePolicy, ManagedLearner},
};

impl<C: Clock> SqliteSupervision<C> {
    /// Enrolls into deny-by-default management. Only explicit operator removal
    /// lifts management; grant revocation never silently removes restrictions.
    pub async fn enroll_parental(
        &self,
        operator: &Operator,
        scope: &Scope,
    ) -> Result<ManagedLearner, Error> {
        operator.require_scope(scope)?;
        let mut op = self.begin().await?;
        if op.load_managed(scope).await?.is_some() {
            return Err(Error::Conflict);
        }
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_supervision_managed")
            .fetch_one(&mut *op.tx)
            .await
            .map_err(storage)?;
        if count >= op.config.limits.managed {
            return Err(Error::Capacity);
        }
        let revision = op.next_revision()?;
        sqlx::query("INSERT INTO rullst_supervision_managed (tenant,subject,resource,revision,operator_ref,evidence_ref) VALUES (?,?,?,?,?,?)")
            .bind(scope.tenant().as_str()).bind(scope.subject().as_str()).bind(scope.resource().as_str()).bind(revision.value())
            .bind(operator.context().actor().as_str()).bind(operator.evidence().as_str()).execute(&mut *op.tx).await.map_err(storage)?;
        op.finish().await?;
        Ok(ManagedLearner {
            scope: scope.clone(),
            revision,
            operator: operator.context().actor().clone(),
            policy_actor: None,
            policy: None,
        })
    }

    pub async fn remove_parental(
        &self,
        operator: &Operator,
        scope: &Scope,
        expected: Revision,
    ) -> Result<(), Error> {
        operator.require_scope(scope)?;
        let mut op = self.begin().await?;
        let current = op.load_managed(scope).await?.ok_or(Error::Forbidden)?;
        if current.revision != expected {
            return Err(Error::Conflict);
        }
        op.next_revision()?;
        let result = sqlx::query("DELETE FROM rullst_supervision_managed WHERE tenant=? AND subject=? AND resource=? AND revision=?")
            .bind(scope.tenant().as_str()).bind(scope.subject().as_str()).bind(scope.resource().as_str()).bind(expected.value())
            .execute(&mut *op.tx).await.map_err(storage)?;
        if result.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        op.finish().await
    }

    pub async fn set_course_policy(
        &self,
        context: &Context,
        scope: &Scope,
        expected: Revision,
        policy: &CoursePolicy,
    ) -> Result<ManagedLearner, Error> {
        let mut op = self.begin().await?;
        op.require_authority(context, scope, AuthorityAction::ParentalManage)
            .await?;
        let mut current = op.load_managed(scope).await?.ok_or(Error::Forbidden)?;
        if current.revision != expected {
            return Err(Error::Conflict);
        }
        if policy.expires_at <= op.now {
            return Err(Error::Expired);
        }
        current.revision = op.next_revision()?;
        current.policy = Some(policy.clone());
        current.policy_actor = Some(context.actor().clone());
        let changed = sqlx::query("UPDATE rullst_supervision_managed SET revision=?,policy_actor=?,not_before=?,expires_at=? WHERE tenant=? AND subject=? AND resource=? AND revision=?")
            .bind(current.revision.value()).bind(context.actor().as_str()).bind(policy.not_before).bind(policy.expires_at)
            .bind(scope.tenant().as_str()).bind(scope.subject().as_str()).bind(scope.resource().as_str()).bind(expected.value())
            .execute(&mut *op.tx).await.map_err(storage)?;
        if changed.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        sqlx::query(
            "DELETE FROM rullst_supervision_courses WHERE tenant=? AND subject=? AND resource=?",
        )
        .bind(scope.tenant().as_str())
        .bind(scope.subject().as_str())
        .bind(scope.resource().as_str())
        .execute(&mut *op.tx)
        .await
        .map_err(storage)?;
        for course in &policy.courses {
            sqlx::query("INSERT INTO rullst_supervision_courses (tenant,subject,resource,course) VALUES (?,?,?,?)")
                .bind(scope.tenant().as_str()).bind(scope.subject().as_str()).bind(scope.resource().as_str()).bind(course.as_str())
                .execute(&mut *op.tx).await.map_err(storage)?;
        }
        op.until(policy.expires_at)?;
        op.finish().await?;
        Ok(current)
    }

    /// Learner or specifically authorized manager view. An absent record is
    /// visible only after identity/authority checks, never via a bare subject ID.
    pub async fn parental(
        &self,
        context: &Context,
        scope: &Scope,
    ) -> Result<Option<ManagedLearner>, Error> {
        let mut op = self.begin().await?;
        if context.require_subject(scope).is_err() {
            op.require_authority(context, scope, AuthorityAction::ParentalManage)
                .await?;
        }
        let result = op.load_managed(scope).await?;
        op.finish().await?;
        Ok(result)
    }

    /// Additional gate for an authenticated learner's original course/lesson
    /// route. Errors deny access. This never establishes enrollment or publication.
    pub async fn learning_access(
        &self,
        context: &Context,
        scope: &Scope,
        course: &OpaqueId,
    ) -> Result<AccessDecision, Error> {
        context.require_subject(scope)?;
        let mut op = self.begin().await?;
        let current = op.load_managed(scope).await?;
        let decision = match current.as_ref().map(|record| record.policy.as_ref()) {
            None => AccessDecision::Unmanaged,
            Some(None) => AccessDecision::MissingPolicy,
            Some(Some(policy)) if op.now < policy.not_before || op.now >= policy.expires_at => {
                AccessDecision::OutsideWindow
            }
            Some(Some(policy)) if !policy.courses.contains(course) => AccessDecision::CourseDenied,
            Some(Some(policy)) => {
                op.until(policy.expires_at)?;
                AccessDecision::Allowed
            }
        };
        op.finish().await?;
        Ok(decision)
    }
}

impl<C: Clock> Operation<'_, C> {
    pub(super) async fn load_managed(
        &mut self,
        scope: &Scope,
    ) -> Result<Option<ManagedLearner>, Error> {
        type Row = (
            i64,
            String,
            String,
            Option<String>,
            Option<i64>,
            Option<i64>,
        );
        let row: Option<Row> = sqlx::query_as("SELECT revision,substr(operator_ref,1,129),substr(evidence_ref,1,129),substr(policy_actor,1,129),not_before,expires_at FROM rullst_supervision_managed WHERE tenant=? AND subject=? AND resource=?")
            .bind(scope.tenant().as_str()).bind(scope.subject().as_str()).bind(scope.resource().as_str()).fetch_optional(&mut *self.tx).await.map_err(storage)?;
        let Some((revision, operator, evidence, actor, not_before, expires_at)) = row else {
            return Ok(None);
        };
        let operator = OpaqueId::new(operator).map_err(|_| Error::Configuration)?;
        OpaqueId::new(evidence).map_err(|_| Error::Configuration)?;
        let courses: Vec<String> = sqlx::query_scalar("SELECT substr(course,1,129) FROM rullst_supervision_courses WHERE tenant=? AND subject=? AND resource=? ORDER BY course LIMIT 65")
            .bind(scope.tenant().as_str()).bind(scope.subject().as_str()).bind(scope.resource().as_str()).fetch_all(&mut *self.tx).await.map_err(storage)?;
        let (policy_actor, policy) = match (actor, not_before, expires_at) {
            (None, None, None) if courses.is_empty() => (None, None),
            (Some(actor), Some(start), Some(end)) => (
                Some(OpaqueId::new(actor).map_err(|_| Error::Configuration)?),
                Some(CoursePolicy::new(courses, start, end).map_err(|_| Error::Configuration)?),
            ),
            _ => return Err(Error::Configuration),
        };
        Ok(Some(ManagedLearner {
            scope: scope.clone(),
            revision: self.check_revision(revision)?,
            operator,
            policy_actor,
            policy,
        }))
    }
}

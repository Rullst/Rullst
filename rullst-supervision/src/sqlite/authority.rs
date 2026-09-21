use super::{SqliteSupervision, storage, transaction::Operation};
use crate::{
    AuthorityAction, AuthorityGrant, AuthorityKey, Clock, Context, OpaqueId, Operator, Revision,
    Scope, SupervisionError as Error, clock::MAX_TIME,
};

impl<C: Clock> SqliteSupervision<C> {
    /// Trusted operator provisioning after independent authority checks. `None`
    /// creates only; replacement requires the exact current revision.
    pub async fn provision_authority(
        &self,
        operator: &Operator,
        key: &AuthorityKey,
        expected: Option<Revision>,
        expires_at: i64,
    ) -> Result<AuthorityGrant, Error> {
        operator.require_scope(key.scope())?;
        let mut op = self.begin().await?;
        if expires_at <= op.now || expires_at > MAX_TIME || expires_at - op.now > 2_592_000 {
            return Err(Error::InvalidInput);
        }
        let old = op.load_grant(key).await?;
        if old.as_ref().map(|grant| grant.revision) != expected {
            return Err(Error::Conflict);
        }
        if old.is_none() {
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_supervision_grants")
                .fetch_one(&mut *op.tx)
                .await
                .map_err(storage)?;
            if count >= op.config.limits.grants {
                return Err(Error::Capacity);
            }
        }
        let revision = op.next_revision()?;
        let scope = key.scope();
        sqlx::query("INSERT INTO rullst_supervision_grants (tenant,subject,resource,delegate,action,revision,expires_at,revoked,operator_ref,evidence_ref) VALUES (?,?,?,?,?,?,?,0,?,?) ON CONFLICT(tenant,subject,resource,delegate,action) DO UPDATE SET revision=excluded.revision,expires_at=excluded.expires_at,revoked=0,operator_ref=excluded.operator_ref,evidence_ref=excluded.evidence_ref")
            .bind(scope.tenant().as_str()).bind(scope.subject().as_str()).bind(scope.resource().as_str())
            .bind(key.delegate().as_str()).bind(key.action().code()).bind(revision.value()).bind(expires_at)
            .bind(operator.context().actor().as_str()).bind(operator.evidence().as_str())
            .execute(&mut *op.tx).await.map_err(storage)?;
        op.until(expires_at)?;
        op.finish().await?;
        Ok(AuthorityGrant {
            key: key.clone(),
            revision,
            expires_at,
            revoked: false,
        })
    }

    /// Exact current administrative state, including expired/revoked records.
    pub async fn authority(
        &self,
        operator: &Operator,
        key: &AuthorityKey,
    ) -> Result<Option<AuthorityGrant>, Error> {
        operator.require_scope(key.scope())?;
        let mut op = self.begin().await?;
        let grant = op.load_grant(key).await?;
        op.finish().await?;
        Ok(grant)
    }

    pub async fn revoke_authority(
        &self,
        operator: &Operator,
        key: &AuthorityKey,
        expected: Revision,
    ) -> Result<AuthorityGrant, Error> {
        operator.require_scope(key.scope())?;
        let mut op = self.begin().await?;
        let mut grant = op.load_grant(key).await?.ok_or(Error::Forbidden)?;
        if grant.revision != expected {
            return Err(Error::Conflict);
        }
        grant.revision = op.next_revision()?;
        grant.revoked = true;
        let scope = key.scope();
        let changed = sqlx::query("UPDATE rullst_supervision_grants SET revision=?,revoked=1,operator_ref=?,evidence_ref=? WHERE tenant=? AND subject=? AND resource=? AND delegate=? AND action=? AND revision=?")
            .bind(grant.revision.value()).bind(operator.context().actor().as_str()).bind(operator.evidence().as_str())
            .bind(scope.tenant().as_str()).bind(scope.subject().as_str()).bind(scope.resource().as_str())
            .bind(key.delegate().as_str()).bind(key.action().code()).bind(expected.value())
            .execute(&mut *op.tx).await.map_err(storage)?;
        if changed.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        op.finish().await?;
        Ok(grant)
    }
}

impl<C: Clock> Operation<'_, C> {
    pub(super) async fn load_grant(
        &mut self,
        key: &AuthorityKey,
    ) -> Result<Option<AuthorityGrant>, Error> {
        let scope = key.scope();
        let row: Option<(i64,i64,i64,String,String)> = sqlx::query_as("SELECT revision,expires_at,revoked,substr(operator_ref,1,129),substr(evidence_ref,1,129) FROM rullst_supervision_grants WHERE tenant=? AND subject=? AND resource=? AND delegate=? AND action=?")
            .bind(scope.tenant().as_str()).bind(scope.subject().as_str()).bind(scope.resource().as_str())
            .bind(key.delegate().as_str()).bind(key.action().code()).fetch_optional(&mut *self.tx).await.map_err(storage)?;
        let Some((revision, expires_at, revoked, operator, evidence)) = row else {
            return Ok(None);
        };
        if !(1..=MAX_TIME).contains(&expires_at) || !matches!(revoked, 0 | 1) {
            return Err(Error::Configuration);
        }
        OpaqueId::new(operator)
            .and_then(|_| OpaqueId::new(evidence))
            .map_err(|_| Error::Configuration)?;
        Ok(Some(AuthorityGrant {
            key: key.clone(),
            revision: self.check_revision(revision)?,
            expires_at,
            revoked: revoked == 1,
        }))
    }

    pub(super) async fn require_authority(
        &mut self,
        context: &Context,
        scope: &Scope,
        action: AuthorityAction,
    ) -> Result<(), Error> {
        if context.tenant() != scope.tenant() {
            return Err(Error::Forbidden);
        }
        let key = AuthorityKey::new(scope.clone(), context.actor().as_str(), action)?;
        let grant = self.load_grant(&key).await?.ok_or(Error::Forbidden)?;
        if grant.revoked || grant.expires_at <= self.now {
            return Err(Error::Forbidden);
        }
        self.until(grant.expires_at)
    }
}

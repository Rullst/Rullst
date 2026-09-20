use super::{SqliteSupervision, storage, transaction::Operation};
use crate::{
    AuthorityAction, Clock, Context, OpaqueId, Revision, Scope, SupervisionError as Error,
    clock::MAX_TIME,
    exam::{Acknowledgement, ExamPolicy, Session, SessionState},
};
use ring::rand::SecureRandom;

impl<C: Clock> SqliteSupervision<C> {
    pub async fn start_exam(
        &self,
        context: &Context,
        scope: &Scope,
        policy: &ExamPolicy,
        acknowledgement: &Acknowledgement,
    ) -> Result<Session, Error> {
        context.require_subject(scope)?;
        acknowledgement.matches(policy.version(), policy.notice())?;
        let mut op = self.begin().await?;
        if policy.lifetime_seconds() > op.config.session_lifetime {
            return Err(Error::InvalidInput);
        }
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_supervision_sessions")
            .fetch_one(&mut *op.tx)
            .await
            .map_err(storage)?;
        if count >= op.config.limits.sessions {
            return Err(Error::Capacity);
        }
        let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_supervision_sessions WHERE tenant=? AND subject=? AND resource=? AND state IN (1,2) AND expires_at>?")
            .bind(scope.tenant().as_str()).bind(scope.subject().as_str()).bind(scope.resource().as_str()).bind(op.now)
            .fetch_one(&mut *op.tx).await.map_err(storage)?;
        if active != 0 {
            return Err(Error::Conflict);
        }
        let mut nonce = [0u8; 32];
        ring::rand::SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| Error::Storage)?;
        let id = OpaqueId::new(
            nonce
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        )?;
        let expires_at = op
            .now
            .checked_add(policy.lifetime_seconds())
            .filter(|value| *value <= MAX_TIME)
            .ok_or(Error::InvalidInput)?;
        let retain_until = expires_at
            .checked_add(op.config.event_retention)
            .filter(|value| *value <= MAX_TIME)
            .ok_or(Error::InvalidInput)?;
        let revision = op.next_revision()?;
        let session = Session {
            id,
            scope: scope.clone(),
            policy: policy.version().clone(),
            notice: policy.notice().clone(),
            state: SessionState::Active,
            revision,
            started_at: op.now,
            expires_at,
            retain_until,
            sequence: 0,
            last_event_at: None,
            event_count: 0,
        };
        sqlx::query("INSERT INTO rullst_supervision_sessions (id,tenant,subject,resource,policy,notice,state,revision,started_at,expires_at,retain_until,sequence,last_event_at,event_count) VALUES (?,?,?,?,?,?,1,?,?,?,?,0,NULL,0)")
            .bind(session.id.as_str()).bind(scope.tenant().as_str()).bind(scope.subject().as_str()).bind(scope.resource().as_str())
            .bind(session.policy.as_str()).bind(session.notice.as_str()).bind(revision.value()).bind(op.now).bind(expires_at).bind(retain_until)
            .execute(&mut *op.tx).await.map_err(storage)?;
        op.until(expires_at)?;
        op.finish().await?;
        Ok(session)
    }

    /// Learner state or a separately authorized reviewer's minimized state.
    pub async fn session(
        &self,
        context: &Context,
        scope: &Scope,
        id: &OpaqueId,
    ) -> Result<Session, Error> {
        let mut op = self.begin().await?;
        op.require_exam_read(context, scope).await?;
        let mut session = op.load_session(scope, id).await?;
        if op.now >= session.expires_at && session.state != SessionState::Ended {
            session.state = SessionState::Expired;
        }
        if matches!(session.state, SessionState::Active | SessionState::Paused) {
            op.until(session.expires_at)?;
        }
        op.until(session.retain_until)?;
        op.finish().await?;
        Ok(session)
    }

    pub async fn pause_exam(
        &self,
        context: &Context,
        scope: &Scope,
        id: &OpaqueId,
        expected: Revision,
    ) -> Result<Session, Error> {
        self.transition(context, scope, id, expected, SessionState::Paused, None)
            .await
    }

    pub async fn resume_exam(
        &self,
        context: &Context,
        scope: &Scope,
        id: &OpaqueId,
        expected: Revision,
        acknowledgement: &Acknowledgement,
    ) -> Result<Session, Error> {
        self.transition(
            context,
            scope,
            id,
            expected,
            SessionState::Active,
            Some(acknowledgement),
        )
        .await
    }

    pub async fn end_exam(
        &self,
        context: &Context,
        scope: &Scope,
        id: &OpaqueId,
        expected: Revision,
    ) -> Result<Session, Error> {
        self.transition(context, scope, id, expected, SessionState::Ended, None)
            .await
    }

    async fn transition(
        &self,
        context: &Context,
        scope: &Scope,
        id: &OpaqueId,
        expected: Revision,
        next: SessionState,
        acknowledgement: Option<&Acknowledgement>,
    ) -> Result<Session, Error> {
        context.require_subject(scope)?;
        let mut op = self.begin().await?;
        let mut session = op.load_session(scope, id).await?;
        op.until(session.expires_at)?;
        if session.revision != expected
            || !matches!(
                (session.state, next),
                (
                    SessionState::Active,
                    SessionState::Paused | SessionState::Ended
                ) | (
                    SessionState::Paused,
                    SessionState::Active | SessionState::Ended
                )
            )
        {
            return Err(Error::Conflict);
        }
        if next == SessionState::Active {
            acknowledgement
                .ok_or(Error::InvalidInput)?
                .matches(&session.policy, &session.notice)?;
        }
        session.revision = op.next_revision()?;
        session.state = next;
        let code = match next {
            SessionState::Active => 1,
            SessionState::Paused => 2,
            SessionState::Ended => 3,
            SessionState::Expired => return Err(Error::InvalidInput),
        };
        let changed = sqlx::query(
            "UPDATE rullst_supervision_sessions SET state=?,revision=? WHERE id=? AND revision=?",
        )
        .bind(code)
        .bind(session.revision.value())
        .bind(id.as_str())
        .bind(expected.value())
        .execute(&mut *op.tx)
        .await
        .map_err(storage)?;
        if changed.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        op.finish().await?;
        Ok(session)
    }
}

impl<C: Clock> Operation<'_, C> {
    pub(super) async fn require_exam_read(
        &mut self,
        context: &Context,
        scope: &Scope,
    ) -> Result<(), Error> {
        if context.require_subject(scope).is_ok() {
            return Ok(());
        }
        self.require_authority(context, scope, AuthorityAction::ExamReview)
            .await
    }

    pub(super) async fn load_session(
        &mut self,
        scope: &Scope,
        id: &OpaqueId,
    ) -> Result<Session, Error> {
        type Row = (
            String,
            String,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            Option<i64>,
            i64,
        );
        let row: Option<Row> = sqlx::query_as("SELECT substr(policy,1,129),substr(notice,1,129),state,revision,started_at,expires_at,retain_until,sequence,last_event_at,event_count FROM rullst_supervision_sessions WHERE id=? AND tenant=? AND subject=? AND resource=?")
            .bind(id.as_str()).bind(scope.tenant().as_str()).bind(scope.subject().as_str()).bind(scope.resource().as_str())
            .fetch_optional(&mut *self.tx).await.map_err(storage)?;
        let Some((
            policy,
            notice,
            state,
            revision,
            started_at,
            expires_at,
            retain_until,
            sequence,
            last_event_at,
            event_count,
        )) = row
        else {
            return Err(Error::Forbidden);
        };
        if started_at < 0
            || started_at > self.now
            || expires_at <= started_at
            || expires_at - started_at > self.config.session_lifetime
            || retain_until > MAX_TIME
            || retain_until <= expires_at
            || retain_until - expires_at != self.config.event_retention
            || sequence < 0
            || sequence != event_count
            || event_count > self.config.limits.events_per_session
            || (sequence == 0) != last_event_at.is_none()
            || last_event_at
                .is_some_and(|time| time < started_at || time >= expires_at || time > self.now)
        {
            return Err(Error::Configuration);
        }
        if self.now >= retain_until {
            return Err(Error::Expired);
        }
        let state = match state {
            1 => SessionState::Active,
            2 => SessionState::Paused,
            3 => SessionState::Ended,
            _ => return Err(Error::Configuration),
        };
        Ok(Session {
            id: id.clone(),
            scope: scope.clone(),
            policy: OpaqueId::new(policy).map_err(|_| Error::Configuration)?,
            notice: OpaqueId::new(notice).map_err(|_| Error::Configuration)?,
            state,
            revision: self.check_revision(revision)?,
            started_at,
            expires_at,
            retain_until,
            sequence,
            last_event_at,
            event_count,
        })
    }
}

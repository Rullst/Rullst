use super::{SqliteSupervision, storage};
use crate::{
    Clock, Context, OpaqueId, Revision, Scope, SupervisionError as Error,
    clock::MAX_TIME,
    exam::{EventReceipt, SessionState, VisibilityEvent},
};

impl<C: Clock> SqliteSupervision<C> {
    /// Accepts only the exact next sequence on an active, current session.
    /// Retries are not silently idempotent: reload state after uncertain outcomes.
    pub async fn record_visibility(
        &self,
        context: &Context,
        scope: &Scope,
        session_id: &OpaqueId,
        revision: Revision,
        sequence: i64,
        event: VisibilityEvent,
    ) -> Result<EventReceipt, Error> {
        context.require_subject(scope)?;
        let mut op = self.begin().await?;
        let session = op.load_session(scope, session_id).await?;
        op.until(session.expires_at)?;
        if session.state != SessionState::Active || session.revision != revision {
            return Err(Error::Conflict);
        }
        if sequence <= 0 || session.sequence.checked_add(1) != Some(sequence) {
            return Err(Error::Sequence);
        }
        if session.event_count >= op.config.limits.events_per_session {
            return Err(Error::Capacity);
        }
        if session
            .last_event_at
            .is_some_and(|time| op.now - time < op.config.limits.event_interval)
        {
            return Err(Error::RateLimited);
        }
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_supervision_events")
            .fetch_one(&mut *op.tx)
            .await
            .map_err(storage)?;
        if count >= op.config.limits.events {
            return Err(Error::Capacity);
        }
        let expires_at = op
            .now
            .checked_add(op.config.event_retention)
            .filter(|time| *time <= MAX_TIME)
            .ok_or(Error::Configuration)?;
        let receipt = EventReceipt {
            sequence,
            event,
            received_at: op.now,
            expires_at,
        };
        let kind = match event {
            VisibilityEvent::PageVisible => 1,
            VisibilityEvent::PageHidden => 2,
        };
        sqlx::query("INSERT INTO rullst_supervision_events (session_id,sequence,kind,received_at,expires_at) VALUES (?,?,?,?,?)")
            .bind(session_id.as_str()).bind(sequence).bind(kind).bind(op.now).bind(expires_at).execute(&mut *op.tx).await.map_err(storage)?;
        sqlx::query("UPDATE rullst_supervision_sessions SET sequence=?,event_count=event_count+1,last_event_at=? WHERE id=?")
            .bind(sequence).bind(op.now).bind(session_id.as_str()).execute(&mut *op.tx).await.map_err(storage)?;
        op.until(expires_at)?;
        op.finish().await?;
        Ok(receipt)
    }

    /// At most 100 non-expired events, with fresh learner/reviewer authorization.
    /// The caller pages by the last returned sequence; missing reports are not
    /// evidence of absence, activity, misconduct or a complete timeline.
    pub async fn events(
        &self,
        context: &Context,
        scope: &Scope,
        session_id: &OpaqueId,
        after_sequence: i64,
        limit: u32,
    ) -> Result<Vec<EventReceipt>, Error> {
        if after_sequence < 0 || !(1..=100).contains(&limit) {
            return Err(Error::InvalidInput);
        }
        let mut op = self.begin().await?;
        op.require_exam_read(context, scope).await?;
        let session = op.load_session(scope, session_id).await?;
        op.until(session.retain_until)?;
        let rows: Vec<(i64,i64,i64,i64)> = sqlx::query_as("SELECT sequence,kind,received_at,expires_at FROM rullst_supervision_events WHERE session_id=? AND sequence>? AND expires_at>? ORDER BY sequence LIMIT ?")
            .bind(session_id.as_str()).bind(after_sequence).bind(op.now).bind(limit).fetch_all(&mut *op.tx).await.map_err(storage)?;
        let mut events = Vec::with_capacity(rows.len());
        for (sequence, kind, received_at, expires_at) in rows {
            if sequence <= after_sequence
                || sequence > session.sequence
                || received_at < session.started_at
                || received_at >= session.expires_at
                || received_at > op.now
                || expires_at > MAX_TIME
                || expires_at - received_at != op.config.event_retention
            {
                return Err(Error::Configuration);
            }
            let event = match kind {
                1 => VisibilityEvent::PageVisible,
                2 => VisibilityEvent::PageHidden,
                _ => return Err(Error::Configuration),
            };
            op.until(expires_at)?;
            events.push(EventReceipt {
                sequence,
                event,
                received_at,
                expires_at,
            });
        }
        op.finish().await?;
        Ok(events)
    }
}

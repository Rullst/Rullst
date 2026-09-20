use super::{SqliteSupervision, storage};
use crate::{
    Clock, Context, OpaqueId, Revision, Scope, SupervisionError as Error,
    clock::MAX_TIME,
    exam::{EventReceipt, VisibilityEvent},
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
        let browser = match event {
            VisibilityEvent::PageVisible => crate::exam::BrowserEvent::PageVisible,
            VisibilityEvent::PageHidden => crate::exam::BrowserEvent::PageHidden,
        };
        let receipt = self
            .record_browser(
                crate::exam::ObservationRequest::new(
                    context, scope, session_id, revision, sequence,
                )?,
                browser,
            )
            .await?;
        Ok(EventReceipt {
            sequence: receipt.sequence(),
            event,
            received_at: receipt.received_at(),
            expires_at: receipt.expires_at(),
        })
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
        let rows: Vec<(i64,i64,i64,i64)> = sqlx::query_as("SELECT sequence,kind,received_at,expires_at FROM rullst_supervision_events WHERE session_id=? AND sequence>? AND expires_at>? AND kind IN (1,2) AND source=1 AND adapter_id IS NULL AND adapter_version IS NULL ORDER BY sequence LIMIT ?")
            .bind(session_id.as_str()).bind(after_sequence).bind(op.now).bind(limit).fetch_all(&mut *op.tx).await.map_err(storage)?;
        let mut events = Vec::with_capacity(rows.len());
        for (sequence, kind, received_at, expires_at) in rows {
            if !session
                .initial_collection
                .contains(crate::exam::Capability::Visibility)
                || sequence <= after_sequence
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

use super::{SqliteSupervision, storage, transaction::Operation};
use crate::{
    Clock, Context, OpaqueId, Revision, Scope, SupervisionError as Error,
    clock::MAX_TIME,
    exam::{
        BrowserEvent, Capability, CaptureDevice, CaptureEvent, Collection, Observation,
        ObservationReceipt, ObservationRequest, ObservationSource, Session, SessionState,
    },
};

impl<C: Clock> SqliteSupervision<C> {
    pub async fn record_browser(
        &self,
        request: ObservationRequest<'_>,
        event: BrowserEvent,
    ) -> Result<ObservationReceipt, Error> {
        self.record_observation(
            request,
            Observation::Browser(event),
            ObservationSource::Browser,
            None,
        )
        .await
    }

    /// Reports capture status only. This does not request/verify browser permission.
    pub async fn record_capture(
        &self,
        request: ObservationRequest<'_>,
        device: CaptureDevice,
        event: CaptureEvent,
    ) -> Result<ObservationReceipt, Error> {
        self.record_observation(
            request,
            Observation::Capture { device, event },
            ObservationSource::Browser,
            None,
        )
        .await
    }

    pub(super) async fn record_observation(
        &self,
        request: ObservationRequest<'_>,
        observation: Observation,
        source: ObservationSource,
        analysis_token: Option<&OpaqueId>,
    ) -> Result<ObservationReceipt, Error> {
        let mut op = self.begin().await?;
        op.authorize_observation(request, observation.capability())
            .await?;
        if let Some(token) = analysis_token {
            let pending: Option<(String, i64, i64, String, i64)> = sqlx::query_as("SELECT substr(token,1,129),revision,sequence,substr(capability,1,129),expires_at FROM rullst_supervision_analysis WHERE session_id=?")
                .bind(request.session.as_str()).fetch_optional(&mut *op.tx).await.map_err(storage)?;
            let Some((stored_token, revision, sequence, capability, expires_at)) = pending else {
                return Err(Error::Conflict);
            };
            if stored_token != token.as_str()
                || revision != request.revision.value()
                || sequence != request.sequence
                || capability != observation.capability().name()
            {
                return Err(Error::Conflict);
            }
            op.until(expires_at)?;
            if !matches!(source, ObservationSource::Adapter { .. }) {
                return Err(Error::InvalidInput);
            }
        } else if !matches!(source, ObservationSource::Browser) {
            return Err(Error::InvalidInput);
        }
        let (source_code, adapter_id, adapter_version) = match &source {
            ObservationSource::Browser
                if matches!(
                    observation,
                    Observation::Browser(_) | Observation::Capture { .. }
                ) =>
            {
                (1, None, None)
            }
            ObservationSource::Adapter {
                id,
                version,
                simulated,
            } if matches!(
                observation,
                Observation::CameraPresence(_) | Observation::AudioActivity(_)
            ) =>
            {
                (
                    if *simulated { 3 } else { 2 },
                    Some(id.as_str()),
                    Some(version.as_str()),
                )
            }
            _ => return Err(Error::InvalidInput),
        };
        let expires_at = op
            .now
            .checked_add(op.config.event_retention)
            .filter(|time| *time <= MAX_TIME)
            .ok_or(Error::Configuration)?;
        let receipt = ObservationReceipt {
            sequence: request.sequence,
            observation,
            source: source.clone(),
            received_at: op.now,
            expires_at,
        };
        sqlx::query("INSERT INTO rullst_supervision_events (session_id,sequence,kind,source,adapter_id,adapter_version,received_at,expires_at) VALUES (?,?,?,?,?,?,?,?)")
            .bind(request.session.as_str()).bind(request.sequence).bind(observation.code()).bind(source_code).bind(adapter_id).bind(adapter_version).bind(op.now).bind(expires_at)
            .execute(&mut *op.tx).await.map_err(storage)?;
        sqlx::query("UPDATE rullst_supervision_sessions SET sequence=?,event_count=event_count+1,last_event_at=? WHERE id=?")
            .bind(request.sequence).bind(op.now).bind(request.session.as_str()).execute(&mut *op.tx).await.map_err(storage)?;
        if analysis_token.is_some() {
            sqlx::query("DELETE FROM rullst_supervision_analysis WHERE session_id=?")
                .bind(request.session.as_str())
                .execute(&mut *op.tx)
                .await
                .map_err(storage)?;
        }
        op.until(expires_at)?;
        op.finish().await?;
        Ok(receipt)
    }

    /// Narrow collection only; enabling a category requires a new session/ack.
    /// Revision fencing rejects pending browser and analysis results immediately
    /// after this transaction commits. It cannot recall already captured media.
    pub async fn restrict_collection(
        &self,
        context: &Context,
        scope: &Scope,
        id: &OpaqueId,
        expected: Revision,
        remaining: Collection,
    ) -> Result<Session, Error> {
        context.require_subject(scope)?;
        let mut op = self.begin().await?;
        let mut session = op.load_session(scope, id).await?;
        op.until(session.expires_at)?;
        if session.revision != expected
            || !matches!(session.state, SessionState::Active | SessionState::Paused)
        {
            return Err(Error::Conflict);
        }
        if !remaining.is_subset_of(session.collection) {
            return Err(Error::Forbidden);
        }
        if remaining == session.collection {
            return Err(Error::Conflict);
        }
        session.revision = op.next_revision()?;
        session.collection = remaining;
        let updated = sqlx::query("UPDATE rullst_supervision_sessions SET collection=?,revision=? WHERE id=? AND revision=?")
            .bind(i64::from(remaining.bits())).bind(session.revision.value()).bind(id.as_str()).bind(expected.value())
            .execute(&mut *op.tx).await.map_err(storage)?;
        if updated.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        op.finish().await?;
        Ok(session)
    }

    /// At most 100 retained observations. No score, penalty or complete-timeline claim.
    pub async fn observations(
        &self,
        context: &Context,
        scope: &Scope,
        id: &OpaqueId,
        after_sequence: i64,
        limit: u32,
    ) -> Result<Vec<ObservationReceipt>, Error> {
        if after_sequence < 0 || !(1..=100).contains(&limit) {
            return Err(Error::InvalidInput);
        }
        let mut op = self.begin().await?;
        op.require_exam_read(context, scope).await?;
        let session = op.load_session(scope, id).await?;
        op.until(session.retain_until)?;
        type Row = (i64, i64, i64, Option<String>, Option<String>, i64, i64);
        let rows: Vec<Row> = sqlx::query_as("SELECT sequence,kind,source,substr(adapter_id,1,129),substr(adapter_version,1,129),received_at,expires_at FROM rullst_supervision_events WHERE session_id=? AND sequence>? AND expires_at>? ORDER BY sequence LIMIT ?")
            .bind(id.as_str()).bind(after_sequence).bind(op.now).bind(limit).fetch_all(&mut *op.tx).await.map_err(storage)?;
        let mut receipts = Vec::with_capacity(rows.len());
        for (sequence, kind, source, adapter_id, adapter_version, received_at, expires_at) in rows {
            if sequence <= after_sequence
                || sequence > session.sequence
                || received_at < session.started_at
                || received_at >= session.expires_at
                || received_at > op.now
                || expires_at > MAX_TIME
                || expires_at.checked_sub(received_at) != Some(op.config.event_retention)
            {
                return Err(Error::Configuration);
            }
            let observation = Observation::from_code(kind)?;
            if !session
                .initial_collection
                .contains(observation.capability())
            {
                return Err(Error::Configuration);
            }
            let source = match (source, adapter_id, adapter_version) {
                (1, None, None)
                    if matches!(
                        observation,
                        Observation::Browser(_) | Observation::Capture { .. }
                    ) =>
                {
                    ObservationSource::Browser
                }
                (code @ (2 | 3), Some(id), Some(version))
                    if matches!(
                        observation,
                        Observation::CameraPresence(_) | Observation::AudioActivity(_)
                    ) =>
                {
                    ObservationSource::Adapter {
                        id: OpaqueId::new(id).map_err(|_| Error::Configuration)?,
                        version: OpaqueId::new(version).map_err(|_| Error::Configuration)?,
                        simulated: code == 3,
                    }
                }
                _ => return Err(Error::Configuration),
            };
            op.until(expires_at)?;
            receipts.push(ObservationReceipt {
                sequence,
                observation,
                source,
                received_at,
                expires_at,
            });
        }
        op.finish().await?;
        Ok(receipts)
    }
}

impl<C: Clock> Operation<'_, C> {
    pub(super) async fn authorize_observation(
        &mut self,
        request: ObservationRequest<'_>,
        capability: Capability,
    ) -> Result<(), Error> {
        request.context.require_subject(request.scope)?;
        let session = self.load_session(request.scope, request.session).await?;
        self.until(session.expires_at)?;
        if session.state != SessionState::Active || session.revision != request.revision {
            return Err(Error::Conflict);
        }
        if !session.collection.contains(capability) {
            return Err(Error::Forbidden);
        }
        if session.sequence.checked_add(1) != Some(request.sequence) {
            return Err(Error::Sequence);
        }
        if session.event_count >= self.config.limits.events_per_session {
            return Err(Error::Capacity);
        }
        if session
            .last_event_at
            .is_some_and(|time| self.now - time < self.config.limits.event_interval)
        {
            return Err(Error::RateLimited);
        }
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_supervision_events")
            .fetch_one(&mut *self.tx)
            .await
            .map_err(storage)?;
        if count >= self.config.limits.events {
            return Err(Error::Capacity);
        }
        Ok(())
    }
}

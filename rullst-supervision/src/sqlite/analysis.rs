use super::{SqliteSupervision, storage};
use crate::{
    Clock, OpaqueId, SupervisionError as Error,
    analysis::{AnalysisAuthorization, AnalysisOptions, Analyzer, MediaSample},
    exam::{ObservationReceipt, ObservationRequest, ObservationSource},
};
use ring::rand::SecureRandom;

impl<C: Clock> SqliteSupervision<C> {
    /// Bounded adapter invocation with one durable pending lease per session.
    /// Requires a Tokio runtime with its timer driver enabled. A dropped future
    /// or uncertain return can retain a lease until its deadline; reload state
    /// before retrying. The host owns media capture and adapter cancellation.
    pub async fn analyze<A: Analyzer, G: AnalysisAuthorization>(
        &self,
        request: ObservationRequest<'_>,
        analyzer: &A,
        authorization: &G,
        sample: MediaSample<'_>,
        options: AnalysisOptions,
    ) -> Result<ObservationReceipt, Error> {
        tokio::runtime::Handle::try_current().map_err(|_| Error::Configuration)?;
        let descriptor = analyzer.descriptor().clone();
        if descriptor.kind() != sample.kind()
            || (descriptor.simulated() && !options.permits_simulation())
        {
            return Err(Error::InvalidInput);
        }
        let operation = async {
            authorization
                .authorize(request.context, request.scope)
                .await?;
            let token = self
                .prepare_analysis(request, descriptor.kind().capability(), options)
                .await?;
            let finding = analyzer.analyze(sample).await?;
            if finding.kind() != descriptor.kind() {
                return Err(Error::InvalidInput);
            }
            authorization
                .authorize(request.context, request.scope)
                .await?;
            self.record_observation(
                request,
                finding.observation(),
                ObservationSource::Adapter {
                    id: descriptor.id().clone(),
                    version: descriptor.version().clone(),
                    simulated: descriptor.simulated(),
                },
                Some(&token),
            )
            .await
        };
        tokio::time::timeout(options.timeout(), operation)
            .await
            .map_err(|_| Error::UncertainCommit)?
    }

    async fn prepare_analysis(
        &self,
        request: ObservationRequest<'_>,
        capability: crate::exam::Capability,
        options: AnalysisOptions,
    ) -> Result<OpaqueId, Error> {
        let mut op = self.begin().await?;
        op.authorize_observation(request, capability).await?;
        let session = op.load_session(request.scope, request.session).await?;
        let deadline = op
            .now
            .checked_add(i64::from(options.timeout_seconds()))
            .ok_or(Error::Clock)?
            .min(session.expires_at());
        op.until(deadline)?;
        sqlx::query("DELETE FROM rullst_supervision_analysis WHERE session_id=? AND expires_at<=?")
            .bind(request.session.as_str())
            .bind(op.now)
            .execute(&mut *op.tx)
            .await
            .map_err(storage)?;
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM rullst_supervision_analysis WHERE session_id=?",
        )
        .bind(request.session.as_str())
        .fetch_one(&mut *op.tx)
        .await
        .map_err(storage)?;
        if exists != 0 {
            return Err(Error::Conflict);
        }
        let mut bytes = [0u8; 32];
        ring::rand::SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| Error::Storage)?;
        let token = OpaqueId::new(
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        )?;
        sqlx::query("INSERT INTO rullst_supervision_analysis (session_id,token,revision,sequence,capability,expires_at) VALUES (?,?,?,?,?,?)")
            .bind(request.session.as_str()).bind(token.as_str()).bind(request.revision.value()).bind(request.sequence).bind(capability.name()).bind(deadline)
            .execute(&mut *op.tx).await.map_err(storage)?;
        op.finish().await?;
        Ok(token)
    }
}

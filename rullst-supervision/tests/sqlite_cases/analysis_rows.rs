//! The pending analysis row binds the exact request that reserved it.
use super::*;
use rullst_supervision::{
    analysis::*,
    exam::{Capability, Collection, ObservationRequest, PresenceObservation},
};
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use std::path::PathBuf;

struct Allow;
impl AnalysisAuthorization for Allow {
    async fn authorize(&self, _: &Context, _: &Scope) -> Result<(), Error> {
        Ok(())
    }
}

/// Changes the pending analysis row while the sample is being analyzed.
struct Tamper {
    descriptor: AnalyzerDescriptor,
    path: PathBuf,
    change: &'static str,
}
impl Analyzer for Tamper {
    fn descriptor(&self) -> &AnalyzerDescriptor {
        &self.descriptor
    }
    async fn analyze(&self, _: MediaSample<'_>) -> Result<Finding, Error> {
        let mut db =
            SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(&self.path))
                .await
                .unwrap();
        sqlx::query(self.change).execute(&mut db).await.unwrap();
        db.close().await.unwrap();
        Ok(Finding::Camera(PresenceObservation::NoPersonDetected))
    }
}

#[tokio::test]
async fn a_changed_pending_analysis_is_never_recorded() {
    for change in [
        "UPDATE rullst_supervision_analysis SET token='another-token'",
        "UPDATE rullst_supervision_analysis SET revision=revision+1",
        "UPDATE rullst_supervision_analysis SET sequence=sequence+1",
        "UPDATE rullst_supervision_analysis SET capability='audio_activity'",
    ] {
        let (temp, store, _) = fixture().await;
        let session = super::observations::start(
            &store,
            Collection::new([Capability::Visibility, Capability::CameraPresence]).unwrap(),
        )
        .await;
        let actor = context("learner-a");
        let scope = scope();
        let request =
            ObservationRequest::new(&actor, &scope, session.id(), session.revision(), 1).unwrap();
        let analyzer = Tamper {
            descriptor: AnalyzerDescriptor::new(
                "local-test-detector",
                "v1",
                AnalysisKind::CameraPresence,
                true,
            )
            .unwrap(),
            path: temp.path().join("supervision.sqlite"),
            change,
        };
        let sample =
            MediaSample::new(SampleFormat::JpegFrame, b"\xff\xd8\xffprivate-media-marker").unwrap();
        let options = AnalysisOptions::new(5)
            .unwrap()
            .allow_simulated_for_testing();
        assert!(
            matches!(
                store
                    .analyze(request, &analyzer, &Allow, sample, options)
                    .await,
                Err(Error::Conflict)
            ),
            "{change}"
        );
    }
}

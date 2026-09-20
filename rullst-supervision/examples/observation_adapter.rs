//! Offline orchestration example, not a camera/audio model or an HTTP endpoint.
//! Run: cargo run -p rullst-supervision --example observation_adapter --features sqlite,analysis
use rullst_supervision::{
    Context, Limits, Scope, StoreConfig, SupervisionError, SystemClock,
    analysis::{
        AnalysisAuthorization, AnalysisKind, AnalysisOptions, Analyzer, AnalyzerDescriptor,
        Finding, MediaSample, SampleFormat,
    },
    exam::{
        Acknowledgement, Capability, Collection, ExamPolicy, ObservationRequest,
        PresenceObservation,
    },
    sqlite::SqliteSupervision,
};

struct DemonstrationAuthorization;
impl AnalysisAuthorization for DemonstrationAuthorization {
    async fn authorize(&self, context: &Context, scope: &Scope) -> Result<(), SupervisionError> {
        // Replace with current authenticated membership, learner entitlement,
        // permission and resource checks. This fixed fixture authenticates nobody.
        if context.tenant().as_str() == "offline-school"
            && context.actor().as_str() == "learner"
            && scope.tenant() == context.tenant()
            && scope.subject() == context.actor()
            && scope.resource().as_str() == "exam"
        {
            Ok(())
        } else {
            Err(SupervisionError::Forbidden)
        }
    }
}
struct SimulatedPresence(AnalyzerDescriptor);
impl Analyzer for SimulatedPresence {
    fn descriptor(&self) -> &AnalyzerDescriptor {
        &self.0
    }
    async fn analyze(&self, _sample: MediaSample<'_>) -> Result<Finding, SupervisionError> {
        // A real implementation can use a local model or a remote provider.
        // It must bound decoding and computation and honor cancellation.
        Ok(Finding::Camera(PresenceObservation::Inconclusive))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let store = SqliteSupervision::initialize(
        directory.path().join("observations.sqlite"),
        StoreConfig::new("offline-example", Limits::new(4, 4, 16, 4)?, 3600, 300)?,
        SystemClock,
    )
    .await?;
    let actor = Context::new("offline-school", "learner")?;
    let scope = Scope::new("offline-school", "learner", "exam")?;
    let collection = Collection::new([Capability::CameraPresence])?;
    // An application supplies these only after showing the exact notice and
    // obtaining the participant's acknowledgement and browser capture permission.
    let policy = ExamPolicy::new("policy-v1", "notice-v1", 300)?.with_collection(collection);
    let ack = Acknowledgement::for_collection("policy-v1", "notice-v1", collection, true)?;
    let session = store
        .start_exam(&actor, &scope, &policy, &ack, None)
        .await?;
    let detector = SimulatedPresence(AnalyzerDescriptor::new(
        "offline-simulation",
        "v1",
        AnalysisKind::CameraPresence,
        true,
    )?);
    let request = ObservationRequest::new(&actor, &scope, session.id(), session.revision(), 1)?;
    // Signature-only synthetic bytes, deliberately not an actual image decoder fixture.
    let sample = MediaSample::new(SampleFormat::JpegFrame, b"\xff\xd8\xffoffline-placeholder")?;
    let receipt = store
        .analyze(
            request,
            &detector,
            &DemonstrationAuthorization,
            sample,
            AnalysisOptions::new(3)?.allow_simulated_for_testing(),
        )
        .await?;
    println!(
        "Simulated observation: {}. No camera was used.",
        receipt.observation().name()
    );
    store
        .end_exam(&actor, &scope, session.id(), session.revision())
        .await?;
    store.close().await;
    Ok(())
}

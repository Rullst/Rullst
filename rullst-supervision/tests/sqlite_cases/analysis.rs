use super::*;
use rullst_supervision::{
    analysis::*,
    exam::{
        AudioObservation, Capability, Collection, Observation, ObservationRequest,
        ObservationSource, PresenceObservation,
    },
};
use tokio::sync::Notify;

struct Guard {
    calls: AtomicUsize,
    deny_on: usize,
}
impl Guard {
    fn allow() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            deny_on: usize::MAX,
        }
    }
}
impl AnalysisAuthorization for Guard {
    async fn authorize(&self, _: &Context, _: &Scope) -> Result<(), Error> {
        if self.calls.fetch_add(1, Ordering::SeqCst) + 1 == self.deny_on {
            Err(Error::Forbidden)
        } else {
            Ok(())
        }
    }
}
struct Detector {
    descriptor: AnalyzerDescriptor,
    finding: Finding,
    calls: AtomicUsize,
    entered: Notify,
    resume: Option<Notify>,
    unavailable: bool,
}
impl Detector {
    fn camera(block: bool) -> Self {
        Self {
            descriptor: AnalyzerDescriptor::new(
                "local-test-detector",
                "v1",
                AnalysisKind::CameraPresence,
                true,
            )
            .unwrap(),
            finding: Finding::Camera(PresenceObservation::NoPersonDetected),
            calls: AtomicUsize::new(0),
            entered: Notify::new(),
            resume: block.then(Notify::new),
            unavailable: false,
        }
    }
}
impl Analyzer for Detector {
    fn descriptor(&self) -> &AnalyzerDescriptor {
        &self.descriptor
    }
    async fn analyze(&self, sample: MediaSample<'_>) -> Result<Finding, Error> {
        assert!(!sample.bytes().is_empty());
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.entered.notify_one();
        if let Some(resume) = &self.resume {
            resume.notified().await;
        }
        if self.unavailable {
            Err(Error::AnalysisUnavailable)
        } else {
            Ok(self.finding)
        }
    }
}
fn sample() -> MediaSample<'static> {
    MediaSample::new(SampleFormat::JpegFrame, b"\xff\xd8\xffprivate-media-marker").unwrap()
}
fn options() -> AnalysisOptions {
    AnalysisOptions::new(5)
        .unwrap()
        .allow_simulated_for_testing()
}
async fn fixture_analysis() -> (
    tempfile::TempDir,
    SqliteSupervision<TestClock>,
    TestClock,
    rullst_supervision::exam::Session,
) {
    let (temp, store, clock) = fixture().await;
    let session = observations::start(
        &store,
        Collection::new([
            Capability::Visibility,
            Capability::CameraPresence,
            Capability::AudioActivity,
        ])
        .unwrap(),
    )
    .await;
    (temp, store, clock, session)
}

#[tokio::test]
async fn analysis_requires_explicit_simulation_selection_and_fresh_authorization_without_retaining_media()
 {
    let (temp, store, clock, session) = fixture_analysis().await;
    let actor = context("learner-a");
    let scope = scope();
    let request =
        ObservationRequest::new(&actor, &scope, session.id(), session.revision(), 1).unwrap();
    let detector = Detector::camera(false);
    let guard = Guard::allow();
    assert!(matches!(
        store
            .analyze(
                request,
                &detector,
                &guard,
                sample(),
                AnalysisOptions::new(5).unwrap()
            )
            .await,
        Err(Error::InvalidInput)
    ));
    assert_eq!(detector.calls.load(Ordering::SeqCst), 0);
    assert_eq!(guard.calls.load(Ordering::SeqCst), 0);
    let denied = Guard {
        calls: AtomicUsize::new(0),
        deny_on: 1,
    };
    assert!(matches!(
        store
            .analyze(request, &detector, &denied, sample(), options())
            .await,
        Err(Error::Forbidden)
    ));
    assert_eq!(detector.calls.load(Ordering::SeqCst), 0);
    let receipt = store
        .analyze(request, &detector, &guard, sample(), options())
        .await
        .unwrap();
    assert_eq!(guard.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        receipt.observation(),
        Observation::CameraPresence(PresenceObservation::NoPersonDetected)
    );
    assert!(
        matches!(receipt.source(),ObservationSource::Adapter {id,version,simulated:true} if id.as_str()=="local-test-detector" && version.as_str()=="v1")
    );
    assert!(
        store
            .events(&actor, &scope, session.id(), 0, 100)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .analyze(request, &detector, &guard, sample(), options())
            .await
            .is_err()
    );
    assert_eq!(detector.calls.load(Ordering::SeqCst), 1);
    store.close().await;
    let store = SqliteSupervision::open(temp.path().join("supervision.sqlite"), config(), clock)
        .await
        .unwrap();
    assert_eq!(
        store
            .observations(&actor, &scope, session.id(), 0, 100)
            .await
            .unwrap(),
        vec![receipt]
    );
    store.close().await;
    for file in std::fs::read_dir(temp.path()).unwrap() {
        let bytes = std::fs::read(file.unwrap().path()).unwrap();
        assert!(
            !bytes
                .windows(b"private-media-marker".len())
                .any(|w| w == b"private-media-marker")
        );
    }
}

#[tokio::test]
async fn pause_end_restriction_and_expiry_reject_a_result_already_in_flight() {
    for action in ["pause", "end", "restrict", "expire", "membership"] {
        let (_temp, store, clock, session) = fixture_analysis().await;
        let actor = context("learner-a");
        let scope = scope();
        let request =
            ObservationRequest::new(&actor, &scope, session.id(), session.revision(), 1).unwrap();
        let detector = Detector::camera(true);
        let guard = Guard {
            calls: AtomicUsize::new(0),
            deny_on: if action == "membership" {
                2
            } else {
                usize::MAX
            },
        };
        let analysis = store.analyze(request, &detector, &guard, sample(), options());
        let revoke = async {
            detector.entered.notified().await;
            match action {
                "pause" => {
                    store
                        .pause_exam(&actor, &scope, session.id(), session.revision())
                        .await
                        .unwrap();
                }
                "end" => {
                    store
                        .end_exam(&actor, &scope, session.id(), session.revision())
                        .await
                        .unwrap();
                }
                "restrict" => {
                    store
                        .restrict_collection(
                            &actor,
                            &scope,
                            session.id(),
                            session.revision(),
                            Collection::visibility_only(),
                        )
                        .await
                        .unwrap();
                }
                "expire" => clock.set(1005),
                "membership" => {}
                _ => unreachable!(),
            }
            detector.resume.as_ref().unwrap().notify_one();
        };
        let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(15), async {
            tokio::join!(analysis, revoke)
        })
        .await
        .expect("revocation fixture must finish even if analysis fails before entry");
        assert!(
            matches!(
                result,
                Err(Error::Conflict | Error::Forbidden | Error::Expired)
            ),
            "{action}: {result:?}"
        );
        assert!(
            store
                .observations(&actor, &scope, session.id(), 0, 100)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            store
                .session(&actor, &scope, session.id())
                .await
                .unwrap()
                .last_sequence(),
            0
        );
    }
}

#[tokio::test]
async fn pending_lease_is_shared_across_openers_and_a_timed_out_attempt_can_only_retry_after_expiry()
 {
    let (temp, store, clock, session) = fixture_analysis().await;
    let second = SqliteSupervision::open(
        temp.path().join("supervision.sqlite"),
        config(),
        clock.clone(),
    )
    .await
    .unwrap();
    let actor = context("learner-a");
    let scope = scope();
    let request =
        ObservationRequest::new(&actor, &scope, session.id(), session.revision(), 1).unwrap();
    let blocked = Detector::camera(true);
    let fast = Detector::camera(false);
    let guard = Guard::allow();
    let short = AnalysisOptions::new(3)
        .unwrap()
        .allow_simulated_for_testing();
    let first = store.analyze(request, &blocked, &guard, sample(), short);
    let contender = async {
        blocked.entered.notified().await;
        assert!(matches!(
            second
                .analyze(request, &fast, &guard, sample(), options())
                .await,
            Err(Error::Conflict)
        ));
        assert_eq!(fast.calls.load(Ordering::SeqCst), 0);
    };
    let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        tokio::join!(first, contender)
    })
    .await
    .expect("lease fixture must finish even if analysis fails before entry");
    assert!(matches!(result, Err(Error::UncertainCommit)));
    assert!(
        store
            .observations(&actor, &scope, session.id(), 0, 10)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        second
            .analyze(request, &fast, &guard, sample(), options())
            .await,
        Err(Error::Conflict)
    ));
    clock.set(1003);
    assert!(
        second
            .analyze(request, &fast, &guard, sample(), options())
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn adapter_failures_wrong_kind_disabled_collection_and_foreign_scope_cannot_emit_findings() {
    let (_temp, store, clock, session) = fixture_analysis().await;
    let actor = context("learner-a");
    let scope = scope();
    let guard = Guard::allow();
    let request =
        ObservationRequest::new(&actor, &scope, session.id(), session.revision(), 1).unwrap();
    let mut detector = Detector::camera(false);
    let foreign_scope = Scope::new("school-a", "learner-a", "other-exam").unwrap();
    let foreign =
        ObservationRequest::new(&actor, &foreign_scope, session.id(), session.revision(), 1)
            .unwrap();
    assert!(matches!(
        store
            .analyze(foreign, &detector, &guard, sample(), options())
            .await,
        Err(Error::Forbidden)
    ));
    assert_eq!(detector.calls.load(Ordering::SeqCst), 0);
    let audio = MediaSample::new(SampleFormat::Pcm16LeMono16Khz, &[0, 0]).unwrap();
    assert!(matches!(
        store
            .analyze(request, &detector, &guard, audio, options())
            .await,
        Err(Error::InvalidInput)
    ));
    assert_eq!(detector.calls.load(Ordering::SeqCst), 0);
    detector.finding = Finding::Audio(AudioObservation::SpeechDetected);
    assert!(matches!(
        store
            .analyze(request, &detector, &guard, sample(), options())
            .await,
        Err(Error::InvalidInput)
    ));
    clock.set(1005);
    detector.unavailable = true;
    assert!(matches!(
        store
            .analyze(request, &detector, &guard, sample(), options())
            .await,
        Err(Error::AnalysisUnavailable)
    ));
    clock.set(1010);
    let restricted = store
        .restrict_collection(
            &actor,
            &scope,
            session.id(),
            session.revision(),
            Collection::visibility_only(),
        )
        .await
        .unwrap();
    let disabled =
        ObservationRequest::new(&actor, &scope, session.id(), restricted.revision(), 1).unwrap();
    assert!(matches!(
        store
            .analyze(disabled, &detector, &guard, sample(), options())
            .await,
        Err(Error::Forbidden)
    ));
    assert_eq!(detector.calls.load(Ordering::SeqCst), 2);
    assert!(
        store
            .observations(&actor, &scope, session.id(), 0, 100)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn audio_and_camera_findings_remain_typed_observations_including_inconclusive_results() {
    let (_temp, store, clock, session) = fixture_analysis().await;
    let actor = context("learner-a");
    let scope = scope();
    let guard = Guard::allow();
    let findings = [
        Finding::Camera(PresenceObservation::PersonDetected),
        Finding::Camera(PresenceObservation::NoPersonDetected),
        Finding::Camera(PresenceObservation::Inconclusive),
        Finding::Audio(AudioObservation::SpeechDetected),
        Finding::Audio(AudioObservation::NoSpeechDetected),
        Finding::Audio(AudioObservation::Inconclusive),
    ];
    for (index, finding) in findings.into_iter().enumerate() {
        clock.set(1000 + index as i64);
        let mut detector = Detector::camera(false);
        detector.descriptor =
            AnalyzerDescriptor::new("bounded-test", "v2", finding.kind(), true).unwrap();
        detector.finding = finding;
        let media = if finding.kind() == AnalysisKind::CameraPresence {
            sample()
        } else {
            MediaSample::new(SampleFormat::Pcm16LeMono16Khz, &[0, 0]).unwrap()
        };
        let request = ObservationRequest::new(
            &actor,
            &scope,
            session.id(),
            session.revision(),
            index as i64 + 1,
        )
        .unwrap();
        let receipt = store
            .analyze(request, &detector, &guard, media, options())
            .await
            .unwrap();
        let expected = match finding {
            Finding::Camera(value) => Observation::CameraPresence(value),
            Finding::Audio(value) => Observation::AudioActivity(value),
            _ => unreachable!(),
        };
        assert_eq!(receipt.observation(), expected);
        assert_eq!(
            store
                .observations(&actor, &scope, session.id(), index as i64, 1)
                .await
                .unwrap(),
            vec![receipt]
        );
    }
}

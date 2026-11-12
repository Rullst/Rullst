//! Loudness-statistics samples: their bounds, and the audio-activity adapter
//! path of `SqliteSupervision::analyze` that records them with no audio.
use super::*;
use rullst_supervision::{
    analysis::*,
    exam::{
        AudioObservation, Capability, Collection, Observation, ObservationReceipt,
        ObservationRequest, ObservationSource, Session,
    },
};

/// Classifies loudness statistics only, like a browser microphone check
/// with 50 ms frames and its own activity threshold.
struct Loudness {
    descriptor: AnalyzerDescriptor,
    calls: AtomicUsize,
}
impl Loudness {
    fn new(kind: AnalysisKind) -> Self {
        Self {
            descriptor: AnalyzerDescriptor::new("loudness-test", "v1", kind, false).unwrap(),
            calls: AtomicUsize::new(0),
        }
    }
}
impl Analyzer for Loudness {
    fn descriptor(&self) -> &AnalyzerDescriptor {
        &self.descriptor
    }
    async fn analyze(&self, sample: MediaSample<'_>) -> Result<Finding, Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(
            sample.bytes().is_empty(),
            "no audio travels with statistics"
        );
        assert_eq!(sample.luminance_statistics(), None);
        let stats = sample
            .loudness_statistics()
            .ok_or(Error::AnalysisUnavailable)?;
        Ok(Finding::Audio(if stats.window_ms() < 2_000 {
            AudioObservation::Inconclusive
        } else if stats.active_fraction() >= 0.15 && stats.peak_dbfs() >= -35.0 {
            AudioObservation::SpeechDetected
        } else if stats.active_fraction() <= 0.03 && stats.peak_dbfs() < -40.0 {
            AudioObservation::NoSpeechDetected
        } else {
            AudioObservation::Inconclusive
        }))
    }
}
struct Allow;
impl AnalysisAuthorization for Allow {
    async fn authorize(&self, _: &Context, _: &Scope) -> Result<(), Error> {
        Ok(())
    }
}

fn loudness(mean: f32, peak: f32, active: f32, window_ms: u32) -> LoudnessStatistics {
    LoudnessStatistics::new(mean, peak, active, window_ms).unwrap()
}

/// Analyzes `sample` as the next event of `session`, two seconds later.
async fn analyze(
    store: &SqliteSupervision<TestClock>,
    clock: &TestClock,
    session: &Session,
    analyzer: &Loudness,
    sample: MediaSample<'_>,
) -> Result<ObservationReceipt, Error> {
    let actor = context("learner-a");
    let scope = scope();
    let current = store.session(&actor, &scope, session.id()).await.unwrap();
    clock.set(clock.time.load(Ordering::SeqCst) + 2);
    let request = ObservationRequest::new(
        &actor,
        &scope,
        session.id(),
        current.revision(),
        current.last_sequence() + 1,
    )
    .unwrap();
    store
        .analyze(
            request,
            analyzer,
            &Allow,
            sample,
            AnalysisOptions::new(5).unwrap(),
        )
        .await
}

async fn names(store: &SqliteSupervision<TestClock>, session: &Session) -> Vec<&'static str> {
    store
        .observations(&context("learner-a"), &scope(), session.id(), 0, 100)
        .await
        .unwrap()
        .iter()
        .map(|receipt| receipt.observation().name())
        .collect()
}

#[test]
fn loudness_statistics_are_bounded_and_carry_no_audio() {
    // Each case breaks exactly one bound.
    let max = LoudnessStatistics::MAX_WINDOW_MS;
    for (mean, peak, active, window_ms) in [
        (-120.1, -10.0, 0.5, max),
        (f32::NAN, -10.0, 0.5, max),
        (f32::NEG_INFINITY, -10.0, 0.5, max),
        (-30.0, 0.1, 0.5, max),
        (-30.0, f32::NAN, 0.5, max),
        (-30.0, f32::INFINITY, 0.5, max),
        (-10.0, -20.0, 0.5, max),
        (-30.0, -10.0, -0.01, max),
        (-30.0, -10.0, 1.01, max),
        (-30.0, -10.0, f32::NAN, max),
        (-30.0, -10.0, 0.5, 0),
        (-30.0, -10.0, 0.5, max + 1),
    ] {
        assert_eq!(
            LoudnessStatistics::new(mean, peak, active, window_ms),
            Err(Error::InvalidInput),
            "{mean} {peak} {active} {window_ms}"
        );
    }
    // The window bound is the five seconds of the longest PCM sample.
    assert_eq!(LoudnessStatistics::MIN_DBFS, -120.0);
    assert_eq!(max, 5_000);
    assert!(
        MediaSample::new(
            SampleFormat::Pcm16LeMono16Khz,
            &vec![0; max as usize * 16 * 2]
        )
        .is_ok()
    );
    // The edges are accepted, a mean equal to the peak included.
    let silent = loudness(-120.0, -120.0, 0.0, 1);
    assert_eq!(
        (
            silent.mean_dbfs(),
            silent.peak_dbfs(),
            silent.active_fraction(),
            silent.window_ms()
        ),
        (-120.0, -120.0, 0.0, 1)
    );
    let full = loudness(0.0, 0.0, 1.0, max);
    assert_eq!(
        (full.mean_dbfs(), full.peak_dbfs(), full.active_fraction()),
        (0.0, 0.0, 1.0)
    );
    let stats = loudness(-41.5, -12.25, 0.375, 4_950);
    assert_eq!(
        (
            stats.mean_dbfs(),
            stats.peak_dbfs(),
            stats.active_fraction(),
            stats.window_ms()
        ),
        (-41.5, -12.25, 0.375, 4_950)
    );

    let sample = MediaSample::loudness(stats);
    assert_eq!(sample.format(), SampleFormat::LoudnessStatistics);
    assert_eq!(sample.kind(), AnalysisKind::AudioActivity);
    assert!(sample.bytes().is_empty());
    assert_eq!(sample.loudness_statistics(), Some(stats));
    assert_eq!(sample.luminance_statistics(), None);
    for bytes in [&b""[..], b"12"] {
        assert!(matches!(
            MediaSample::new(SampleFormat::LoudnessStatistics, bytes),
            Err(Error::InvalidInput)
        ));
    }
    let audio = MediaSample::new(SampleFormat::Pcm16LeMono16Khz, &[0, 0]).unwrap();
    assert_eq!(audio.kind(), AnalysisKind::AudioActivity);
    assert_eq!(audio.loudness_statistics(), None);
    assert_eq!(audio.luminance_statistics(), None);
}

#[tokio::test]
async fn loudness_findings_are_recorded_through_an_audio_activity_adapter() {
    let (_temp, store, clock) = fixture().await;
    let session = observations::start(
        &store,
        Collection::new([Capability::AudioActivity]).unwrap(),
    )
    .await;
    let adapter = Loudness::new(AnalysisKind::AudioActivity);
    for (stats, expected) in [
        (
            loudness(-31.5, -12.0, 0.6, 5_000),
            AudioObservation::SpeechDetected,
        ),
        (
            loudness(-62.0, -50.0, 0.0, 5_000),
            AudioObservation::NoSpeechDetected,
        ),
        (
            loudness(-31.5, -12.0, 0.6, 1_000),
            AudioObservation::Inconclusive,
        ),
    ] {
        let receipt = analyze(
            &store,
            &clock,
            &session,
            &adapter,
            MediaSample::loudness(stats),
        )
        .await
        .unwrap();
        assert_eq!(receipt.observation(), Observation::AudioActivity(expected));
        assert!(matches!(
            receipt.source(),
            ObservationSource::Adapter { id, version, simulated: false }
                if id.as_str() == "loudness-test" && version.as_str() == "v1"
        ));
    }
    assert_eq!(adapter.calls.load(Ordering::SeqCst), 3);
    assert_eq!(
        names(&store, &session).await,
        [
            "speech_detected",
            "no_speech_detected",
            "audio_inconclusive"
        ]
    );
}

#[tokio::test]
async fn statistics_samples_are_refused_by_an_adapter_of_the_other_kind() {
    let (_temp, store, clock) = fixture().await;
    let session = observations::start(
        &store,
        Collection::new([Capability::CameraPresence, Capability::AudioActivity]).unwrap(),
    )
    .await;
    let camera = Loudness::new(AnalysisKind::CameraPresence);
    let audio = Loudness::new(AnalysisKind::AudioActivity);
    let speech = loudness(-31.5, -12.0, 0.6, 5_000);
    let brightness = LuminanceStatistics::new(120.0, 30.0, 192).unwrap();
    assert_eq!(
        analyze(
            &store,
            &clock,
            &session,
            &camera,
            MediaSample::loudness(speech)
        )
        .await
        .unwrap_err(),
        Error::InvalidInput
    );
    assert_eq!(
        analyze(
            &store,
            &clock,
            &session,
            &audio,
            MediaSample::luminance(brightness)
        )
        .await
        .unwrap_err(),
        Error::InvalidInput
    );
    assert_eq!(camera.calls.load(Ordering::SeqCst), 0);
    assert_eq!(audio.calls.load(Ordering::SeqCst), 0);
    assert!(names(&store, &session).await.is_empty());
    // The same session and sequence take the sample of the adapter's kind.
    assert_eq!(
        analyze(
            &store,
            &clock,
            &session,
            &audio,
            MediaSample::loudness(speech)
        )
        .await
        .unwrap()
        .observation(),
        Observation::AudioActivity(AudioObservation::SpeechDetected)
    );
    assert_eq!(names(&store, &session).await, ["speech_detected"]);
}

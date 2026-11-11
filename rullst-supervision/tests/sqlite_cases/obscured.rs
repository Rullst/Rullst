//! `PresenceObservation::Obscured`, brightness-statistics samples and the
//! opt-in event-table rebuild of stores created before 13.0.0-alpha.5.
use super::*;
use rullst_supervision::{
    analysis::*,
    exam::{Capability, Collection, Observation, ObservationRequest, PresenceObservation},
};
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};

/// The events table exactly as stores created before 13.0.0-alpha.5 have it.
const LEGACY_EVENTS: &str = "CREATE TABLE rullst_supervision_events (session_id TEXT NOT NULL REFERENCES rullst_supervision_sessions(id) ON DELETE CASCADE, sequence INTEGER NOT NULL CHECK(sequence > 0), kind INTEGER NOT NULL CHECK(kind BETWEEN 1 AND 27), source INTEGER NOT NULL DEFAULT 1 CHECK(source IN (1,2,3)), adapter_id TEXT, adapter_version TEXT, received_at INTEGER NOT NULL CHECK(received_at >= 0), expires_at INTEGER NOT NULL CHECK(expires_at > received_at), PRIMARY KEY(session_id,sequence))";

/// Classifies brightness statistics only: dark or flat means obscured.
struct Brightness(AnalyzerDescriptor);
impl Analyzer for Brightness {
    fn descriptor(&self) -> &AnalyzerDescriptor {
        &self.0
    }
    async fn analyze(&self, sample: MediaSample<'_>) -> Result<Finding, Error> {
        assert!(
            sample.bytes().is_empty(),
            "no image travels with statistics"
        );
        let stats = sample.statistics().ok_or(Error::AnalysisUnavailable)?;
        Ok(Finding::Camera(
            if stats.mean() < 24.0 || stats.standard_deviation() < 6.0 {
                PresenceObservation::Obscured
            } else {
                PresenceObservation::Inconclusive
            },
        ))
    }
}
struct Allow;
impl AnalysisAuthorization for Allow {
    async fn authorize(&self, _: &Context, _: &Scope) -> Result<(), Error> {
        Ok(())
    }
}

fn detector() -> Brightness {
    Brightness(
        AnalyzerDescriptor::new("luma-test", "v1", AnalysisKind::CameraPresence, false).unwrap(),
    )
}

async fn record(
    store: &SqliteSupervision<TestClock>,
    clock: &TestClock,
    session: &rullst_supervision::exam::Session,
    mean: f32,
) -> Result<Observation, Error> {
    let current = store
        .session(&context("learner-a"), &scope(), session.id())
        .await
        .unwrap();
    clock.set(clock.time.load(Ordering::SeqCst) + 2);
    let actor = context("learner-a");
    let scope = scope();
    let request = ObservationRequest::new(
        &actor,
        &scope,
        session.id(),
        current.revision(),
        current.last_sequence() + 1,
    )
    .unwrap();
    let sample = MediaSample::luminance(LuminanceStatistics::new(mean, 30.0, 192).unwrap());
    store
        .analyze(
            request,
            &detector(),
            &Allow,
            sample,
            AnalysisOptions::new(5).unwrap(),
        )
        .await
        .map(|receipt| receipt.observation())
}

async fn names(store: &SqliteSupervision<TestClock>, session: &OpaqueId) -> Vec<&'static str> {
    store
        .observations(&context("learner-a"), &scope(), session, 0, 100)
        .await
        .unwrap()
        .iter()
        .map(|receipt| receipt.observation().name())
        .collect()
}

#[test]
fn luminance_statistics_are_bounded_and_carry_no_bytes() {
    for (mean, deviation, samples) in [
        (-0.1, 1.0, 1),
        (255.1, 1.0, 1),
        (f32::NAN, 1.0, 1),
        (10.0, 127.6, 1),
        (10.0, f32::INFINITY, 1),
        (10.0, 1.0, 0),
        (10.0, 1.0, LuminanceStatistics::MAX_SAMPLES + 1),
    ] {
        assert_eq!(
            LuminanceStatistics::new(mean, deviation, samples),
            Err(Error::InvalidInput)
        );
    }
    let stats = LuminanceStatistics::new(255.0, 127.5, LuminanceStatistics::MAX_SAMPLES).unwrap();
    let sample = MediaSample::luminance(stats);
    assert_eq!(sample.format(), SampleFormat::LuminanceStatistics);
    assert_eq!(sample.kind(), AnalysisKind::CameraPresence);
    assert!(sample.bytes().is_empty());
    assert_eq!(sample.statistics(), Some(stats));
    assert!(matches!(
        MediaSample::new(SampleFormat::LuminanceStatistics, b"12"),
        Err(Error::InvalidInput)
    ));
    let frame = MediaSample::new(SampleFormat::PngFrame, b"\x89PNG\r\n\x1a\nx").unwrap();
    assert_eq!(frame.statistics(), None);
}

#[tokio::test]
async fn obscured_findings_from_statistics_are_recorded_and_read_back() {
    let (_temp, store, clock) = fixture().await;
    let session = observations::start(
        &store,
        Collection::new([Capability::CameraPresence]).unwrap(),
    )
    .await;
    assert_eq!(
        record(&store, &clock, &session, 4.0).await.unwrap(),
        Observation::CameraPresence(PresenceObservation::Obscured)
    );
    assert_eq!(
        record(&store, &clock, &session, 120.0).await.unwrap(),
        Observation::CameraPresence(PresenceObservation::Inconclusive)
    );
    assert_eq!(
        names(&store, session.id()).await,
        ["camera_obscured", "presence_inconclusive"]
    );
}

/// Turns a fresh store's events table back into the pre-alpha.5 definition.
async fn make_legacy(path: &std::path::Path) {
    let mut connection =
        SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(path))
            .await
            .unwrap();
    for statement in [
        "ALTER TABLE rullst_supervision_events RENAME TO old_events",
        LEGACY_EVENTS,
        "INSERT INTO rullst_supervision_events SELECT * FROM old_events",
        "DROP TABLE old_events",
        "CREATE INDEX rullst_supervision_event_expiry ON rullst_supervision_events(expires_at)",
    ] {
        sqlx::query(statement)
            .execute(&mut connection)
            .await
            .unwrap();
    }
    connection.close().await.unwrap();
}

#[tokio::test]
async fn a_store_from_before_alpha5_keeps_working_and_upgrades_once_on_request() {
    let (temp, store, clock) = fixture().await;
    store.close().await;
    let path = temp.path().join("supervision.sqlite");
    make_legacy(&path).await;
    let store = SqliteSupervision::open(&path, config(), clock.clone())
        .await
        .unwrap();
    let session = observations::start(
        &store,
        Collection::new([Capability::CameraPresence]).unwrap(),
    )
    .await;
    assert_eq!(
        record(&store, &clock, &session, 120.0).await.unwrap(),
        Observation::CameraPresence(PresenceObservation::Inconclusive)
    );
    // The narrower table cannot hold the new value: refused, nothing written.
    assert_eq!(
        record(&store, &clock, &session, 4.0).await.unwrap_err(),
        Error::Configuration
    );
    assert_eq!(names(&store, session.id()).await, ["presence_inconclusive"]);
    // The refused attempt's analysis lease (5 s) must run out first.
    clock.set(clock.time.load(Ordering::SeqCst) + 10);

    assert!(store.upgrade_event_schema().await.unwrap());
    assert!(!store.upgrade_event_schema().await.unwrap());
    assert_eq!(names(&store, session.id()).await, ["presence_inconclusive"]);
    assert_eq!(
        record(&store, &clock, &session, 4.0).await.unwrap(),
        Observation::CameraPresence(PresenceObservation::Obscured)
    );
    store.close().await;
    // The rebuilt store opens as a current one, with its rows.
    let reopened = SqliteSupervision::open(&path, config(), clock)
        .await
        .unwrap();
    assert_eq!(
        names(&reopened, session.id()).await,
        ["presence_inconclusive", "camera_obscured"]
    );
    assert!(!reopened.upgrade_event_schema().await.unwrap());
}

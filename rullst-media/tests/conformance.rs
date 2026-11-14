//! The video conformance kit against the offline Bunny Stream adapter and
//! against wrappers that break (or keep) one rule each.
#![cfg(all(feature = "conformance", feature = "bunny"))]

use rullst_media::bunny::{BunnyConfig, BunnyCredentials, BunnyStream, PrivateDelivery};
use rullst_media::conformance::{
    CaseOutcome, ConformanceReport, VideoExpectations, check_video_provider,
};
use rullst_media::*;
use std::sync::atomic::{AtomicU32, Ordering};

const NOW: i64 = 1_793_534_400;

fn full() -> BunnyStream {
    let config = BunnyConfig::new(
        LibraryId::new(7).unwrap(),
        Reference::new("conformance").unwrap(),
        "conformance.b-cdn.net",
        BunnyCredentials::new("mock_api", "mock_webhook", "mock_embed", "mock_cdn").unwrap(),
        PrivateDelivery::configured(true, true, true).unwrap(),
    )
    .unwrap();
    BunnyStream::new(config).unwrap()
}

fn upload_only() -> BunnyStream {
    let config = BunnyConfig::upload_only(
        LibraryId::new(7).unwrap(),
        Reference::new("conformance").unwrap(),
        BunnyCredentials::upload_only("").unwrap(),
    )
    .unwrap();
    BunnyStream::new(config).unwrap()
}

fn outcome(report: &ConformanceReport, id: &str) -> CaseOutcome {
    report.case(id).map(|case| case.outcome()).unwrap()
}

#[tokio::test]
async fn offline_bunny_passes_every_case() {
    let report = check_video_provider("bunny", &full(), VideoExpectations::new(NOW)).await;
    report.assert_passed();
    assert_eq!(report.provider(), "bunny");
    assert_eq!(report.cases().len(), 8);
    for case in report.cases() {
        assert_eq!(case.outcome(), CaseOutcome::Passed, "{}", case.id());
    }
}

#[tokio::test]
async fn upload_only_credentials_refuse_playback() {
    let expect = VideoExpectations::new(NOW).without_playback();
    let report = check_video_provider("bunny", &upload_only(), expect).await;
    report.assert_passed();
    assert_eq!(
        outcome(&report, "offline.deterministic"),
        CaseOutcome::Passed
    );

    let report = check_video_provider("bunny", &upload_only(), VideoExpectations::new(NOW)).await;
    assert_eq!(outcome(&report, "playback.grant"), CaseOutcome::Failed);
    assert_eq!(
        report.case("playback.grant").unwrap().detail(),
        "at least one playback kind is granted"
    );
}

#[tokio::test]
async fn a_capability_declared_absent_must_be_absent() {
    for (expect, id) in [
        (VideoExpectations::new(NOW).without_upload(), "upload.grant"),
        (
            VideoExpectations::new(NOW).without_playback(),
            "playback.grant",
        ),
        (
            VideoExpectations::new(NOW).without_listing(),
            "list.bounded",
        ),
    ] {
        let report = check_video_provider("bunny", &full(), expect).await;
        assert_eq!(outcome(&report, id), CaseOutcome::Failed, "{report}");
        assert_eq!(report.failures().count(), 1, "{report}");
    }
}

/// One rule a wrapper breaks, or one conforming variation it adds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Twist {
    Clean,
    UnstableBinding,
    RemoteMode,
    AcceptsBadMarker,
    LosesUpdate,
    KeepsDeleted,
    DeleteNotIdempotent,
    Panics,
    WrongExpiry,
    HttpEndpoint,
    AcceptsZeroTtl,
    DeclaredUpload,
    BadDeclaredUpload,
    NoPlayback,
    LongPlayback,
    FullPage,
    OverfullPage,
    WrongPageNumber,
    NoListing,
    NonDeterministicGrant,
    IgnoresClock,
}

/// An instrumentation-style wrapper around the offline Bunny adapter.
struct Probe {
    inner: BunnyStream,
    twist: Twist,
    calls: AtomicU32,
}

impl Probe {
    fn new(twist: Twist) -> Self {
        Self {
            inner: full(),
            twist,
            calls: AtomicU32::new(0),
        }
    }
    fn tick(&self) -> u32 {
        self.calls.fetch_add(1, Ordering::Relaxed)
    }
}

impl VideoProvider for Probe {
    fn binding(&self) -> ProviderBinding {
        let mut binding = self.inner.binding();
        match self.twist {
            Twist::UnstableBinding if self.tick() % 2 == 1 => {
                binding.environment = Reference::new("moved").unwrap();
            }
            Twist::RemoteMode => binding.mode = ProviderMode::RemoteUnvalidated,
            _ => {}
        }
        binding
    }
    async fn create(&self, marker: &str) -> Result<RemoteVideo, MediaError> {
        if self.twist == Twist::AcceptsBadMarker && marker.len() != 45 {
            return self
                .inner
                .create("rullst-video-0000000000000000000000000000beef")
                .await;
        }
        self.inner.create(marker).await
    }
    async fn find_created(&self, marker: &str) -> Result<Option<RemoteVideo>, MediaError> {
        self.inner.find_created(marker).await
    }
    async fn get(&self, video: &VideoId) -> Result<Option<RemoteVideo>, MediaError> {
        if self.twist == Twist::Panics {
            panic!("probe get panic");
        }
        self.inner.get(video).await
    }
    async fn update(&self, video: &VideoId, metadata: &Metadata) -> Result<(), MediaError> {
        match self.twist {
            Twist::LosesUpdate => Ok(()),
            _ => self.inner.update(video, metadata).await,
        }
    }
    async fn delete(&self, video: &VideoId) -> Result<(), MediaError> {
        if self.twist == Twist::KeepsDeleted {
            return Ok(());
        }
        if self.twist == Twist::DeleteNotIdempotent && self.inner.get(video).await == Ok(None) {
            return Err(MediaError::NotFound);
        }
        self.inner.delete(video).await
    }
    fn upload(&self, video: &VideoId, now: i64, ttl: u32) -> Result<UploadGrant, MediaError> {
        let (now, ttl) = match self.twist {
            Twist::AcceptsZeroTtl => (now, ttl.max(1)),
            Twist::IgnoresClock => (NOW, 600),
            _ => (now, ttl),
        };
        let mut grant = self.inner.upload(video, now, ttl)?;
        match self.twist {
            Twist::WrongExpiry => grant.expires_at += 1,
            Twist::HttpEndpoint => grant.endpoint = grant.endpoint.replacen("https", "http", 1),
            Twist::NonDeterministicGrant => {
                grant.endpoint = format!("{}?n={}", grant.endpoint, self.tick())
            }
            _ => {}
        }
        Ok(grant)
    }
    fn upload_declared(
        &self,
        video: &VideoId,
        now: i64,
        ttl: u32,
        declaration: &UploadDeclaration,
    ) -> Result<UploadGrant, MediaError> {
        if !matches!(self.twist, Twist::DeclaredUpload | Twist::BadDeclaredUpload) {
            return self.inner.upload_declared(video, now, ttl, declaration);
        }
        let mut grant = self.inner.upload(video, now, ttl)?;
        grant.protocol = UploadProtocol::PresignedPut;
        grant.content_type = Some(declaration.content_type().to_owned());
        let extra = u64::from(self.twist == Twist::BadDeclaredUpload);
        grant.content_length = Some(declaration.length() + extra);
        Ok(grant)
    }
    fn playback(
        &self,
        video: &VideoId,
        now: i64,
        ttl: u32,
        kind: PlaybackKind,
    ) -> Result<PlaybackGrant, MediaError> {
        if self.twist == Twist::NoPlayback {
            return Err(MediaError::Unsupported);
        }
        let mut grant = self.inner.playback(video, now, ttl, kind)?;
        if self.twist == Twist::LongPlayback {
            grant.expires_at += 1;
        }
        Ok(grant)
    }
    async fn list(&self, query: &ListQuery) -> Result<VideoPage, MediaError> {
        if self.twist == Twist::NoListing {
            return Err(MediaError::Unsupported);
        }
        let mut page = self.inner.list(query).await?;
        match self.twist {
            Twist::FullPage | Twist::OverfullPage => {
                let first = self.inner.list(&ListQuery::new(1, 10)?).await?;
                let fill =
                    query.per_page() as usize + usize::from(self.twist == Twist::OverfullPage);
                while page.items.len() < fill {
                    page.items.push(first.items[0].clone());
                }
                page.total = u64::from(query.page() * query.per_page());
            }
            Twist::WrongPageNumber => page.page += 1,
            _ => {}
        }
        Ok(page)
    }
}

async fn run(twist: Twist, expect: VideoExpectations) -> ConformanceReport {
    check_video_provider("probe", &Probe::new(twist), expect).await
}

#[tokio::test]
async fn conforming_wrappers_pass() {
    for (twist, expect) in [
        (Twist::Clean, VideoExpectations::new(NOW)),
        (Twist::FullPage, VideoExpectations::new(NOW)),
        (Twist::DeclaredUpload, VideoExpectations::new(NOW)),
        (
            Twist::NoPlayback,
            VideoExpectations::new(NOW).without_playback(),
        ),
        (
            Twist::NoListing,
            VideoExpectations::new(NOW).without_listing(),
        ),
    ] {
        let report = run(twist, expect).await;
        assert!(report.passed(), "{twist:?}\n{report}");
    }
    let report = run(Twist::RemoteMode, VideoExpectations::new(NOW)).await;
    assert_eq!(
        outcome(&report, "offline.deterministic"),
        CaseOutcome::Skipped
    );
    assert_eq!(
        report.case("offline.deterministic").unwrap().detail(),
        "the provider is not in offline mode"
    );
}

#[tokio::test]
async fn each_broken_rule_fails_its_case() {
    use Twist::*;
    for (twist, id) in [
        (UnstableBinding, "binding.stable"),
        (AcceptsBadMarker, "create.marker_validated"),
        (LosesUpdate, "lifecycle.create_get_update_delete"),
        (KeepsDeleted, "lifecycle.create_get_update_delete"),
        (DeleteNotIdempotent, "lifecycle.create_get_update_delete"),
        (Panics, "lifecycle.create_get_update_delete"),
        (WrongExpiry, "upload.grant"),
        (HttpEndpoint, "upload.grant"),
        (AcceptsZeroTtl, "upload.grant"),
        (BadDeclaredUpload, "upload.grant"),
        (NoPlayback, "playback.grant"),
        (LongPlayback, "playback.grant"),
        (OverfullPage, "list.bounded"),
        (WrongPageNumber, "list.bounded"),
        (NoListing, "list.bounded"),
        (NonDeterministicGrant, "offline.deterministic"),
        (IgnoresClock, "errors.typed_no_panic"),
    ] {
        let report = run(twist, VideoExpectations::new(NOW)).await;
        assert_eq!(
            outcome(&report, id),
            CaseOutcome::Failed,
            "{twist:?}\n{report}"
        );
    }
}

#[tokio::test]
async fn the_report_lists_every_case() {
    let report = run(Twist::Panics, VideoExpectations::new(NOW)).await;
    assert_eq!(
        report
            .case("lifecycle.create_get_update_delete")
            .unwrap()
            .detail(),
        "panicked: probe get panic"
    );

    let report = run(Twist::WrongPageNumber, VideoExpectations::new(NOW)).await;
    let text = report.to_string();
    assert!(
        text.starts_with(
            "conformance of probe: 7 passed, 1 failed, 0 skipped\n  ok binding.stable\n"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "\n  FAILED list.bounded: page 1 echoes its number; page 2 echoes its number\n"
        ),
        "{text}"
    );
    assert_eq!(
        report.failures().map(|case| case.id()).collect::<Vec<_>>(),
        ["list.bounded"]
    );
    assert!(report.case("no.such.case").is_none());

    let skipped = run(Twist::RemoteMode, VideoExpectations::new(NOW))
        .await
        .to_string();
    assert!(
        skipped
            .contains("\n  skipped offline.deterministic: the provider is not in offline mode\n"),
        "{skipped}"
    );
}

#[tokio::test]
#[should_panic(expected = "FAILED binding.stable")]
async fn assert_passed_fails_the_test_with_the_report() {
    run(Twist::UnstableBinding, VideoExpectations::new(NOW))
        .await
        .assert_passed();
}

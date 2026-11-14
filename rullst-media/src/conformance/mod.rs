//! Conformance kit for [`VideoProvider`] implementations (Cargo feature
//! `conformance`). Since 13.0.0-alpha.5.
//!
//! An application-owned video adapter, or an instrumentation wrapper around a
//! built-in one, proves it behaves like the Bunny Stream adapter by passing
//! [`check_video_provider`] in its tests:
//!
//! ```ignore
//! let report = check_video_provider("bunny", &provider, VideoExpectations::new(now)).await;
//! report.assert_passed();
//! ```
//!
//! Run it against the offline mode (empty or `mock_*` keys) or a disposable
//! library: it creates, renames and deletes videos. [`VideoExpectations`]
//! states which capabilities the provider offers; a capability declared
//! absent must fail closed with [`MediaError::Unsupported`].
//!
//! | Case | What a conforming provider does |
//! | --- | --- |
//! | `binding.stable` | `binding()` returns the same binding every time |
//! | `create.marker_validated` | `create` and `find_created` refuse a malformed marker with `InvalidInput` |
//! | `lifecycle.create_get_update_delete` | a created video is found, renamed, deleted, and deleting it again succeeds |
//! | `upload.grant` | an upload grant names the video, library and mode, expires after its TTL, is signed and refuses invalid TTLs and clocks; a declared upload binds its type and length or is `Unsupported` |
//! | `playback.grant` | playback grants expire after their TTL over HTTPS, or are `Unsupported`; all `Unsupported` without playback (upload-only credentials) |
//! | `list.bounded` | a page echoes its number and size and holds at most `per_page` videos |
//! | `offline.deterministic` | in [`ProviderMode::Offline`], the same grant and listing requests give the same answers |
//! | `errors.typed_no_panic` | unknown videos and impossible clocks give typed results, never a panic |
//!
//! Every case runs under `catch_unwind`: a panic fails that case, never the
//! run.
//!
//! [`ProviderMode::Offline`]: crate::ProviderMode::Offline
//! [`MediaError::Unsupported`]: crate::MediaError::Unsupported

mod cases;
mod report;

pub use report::{CaseOutcome, ConformanceCase, ConformanceReport};

use crate::VideoProvider;
use report::run_case;

/// The capabilities a provider under test offers, and the trusted clock its
/// grants are checked against. Since 13.0.0-alpha.5.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VideoExpectations {
    now: i64,
    upload: bool,
    playback: bool,
    listing: bool,
}

impl VideoExpectations {
    /// Upload grants, playback grants and library listing, checked at `now`
    /// (UTC Unix seconds).
    pub fn new(now: i64) -> Self {
        Self {
            now,
            upload: true,
            playback: true,
            listing: true,
        }
    }

    /// The provider grants no uploads: `upload` must be `Unsupported`.
    pub fn without_upload(mut self) -> Self {
        self.upload = false;
        self
    }

    /// The provider grants no playback, as with Bunny's upload-only
    /// credentials: every `playback` kind must be `Unsupported`.
    pub fn without_playback(mut self) -> Self {
        self.playback = false;
        self
    }

    /// The provider lists no library: `list` must be `Unsupported`.
    pub fn without_listing(mut self) -> Self {
        self.listing = false;
        self
    }
}

/// Runs every case of the video conformance kit against `provider`.
/// Since 13.0.0-alpha.5.
///
/// It never panics: a panic inside the provider fails the case it happened
/// in. `name` labels the report.
pub async fn check_video_provider<P: VideoProvider>(
    name: impl Into<String>,
    provider: &P,
    expect: VideoExpectations,
) -> ConformanceReport {
    let run = cases::Run { provider, expect };
    let cases = vec![
        run_case("binding.stable", run.binding_stable()).await,
        run_case("create.marker_validated", run.marker_validated()).await,
        run_case("lifecycle.create_get_update_delete", run.lifecycle()).await,
        run_case("upload.grant", run.upload_grant()).await,
        run_case("playback.grant", run.playback_grant()).await,
        run_case("list.bounded", run.list_bounded()).await,
        run_case("offline.deterministic", run.deterministic()).await,
        run_case("errors.typed_no_panic", run.typed_errors()).await,
    ];
    ConformanceReport::new(name, cases)
}

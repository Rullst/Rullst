# Private course videos with Bunny Stream

`rullst-media` is an optional, unpublished v13 candidate. It implements a bounded
private-video lifecycle: authorized creation and resumable upload, processing
refresh, explicit publication, current-entitlement playback, withdrawal, metadata
updates, deletion and durable reconciliation. It is not yet admitted to the
release inventory or framework facade. Existing blueprints are not changed.

The [crate integration guide](../../rullst-media/README.md) describes installation
from the candidate checkout, separate provider keys, required embed/CDN/direct-file
protection, HTTP security composition, browser uploads, captions/transcripts,
typed failure handling, SQLite retention and backup responsibilities.

Run `cargo run -p rullst-media --example private_course_video --all-features`
to exercise the executable offline journey. Empty or `mock_*` credentials never
become live playback authority; production store configuration rejects them.
Applications supply authenticated tenant/course membership and current entitlement.

Automated acceptance uses actual local HTTP, SQLite and Chromium with controlled
media. It checks signatures, upload bytes and failure recovery, CSRF and tenant
denial, media decoding, captions/keyboard interaction, withdrawal and deletion.
Real Bunny account, transcoding, CDN and iframe interoperability remain unvalidated
under the owner's no-live-provider-testing instruction. Signed URLs do not imply
DRM, immediate revocation of existing capabilities or prevention of all copying.

See the [managed-video acceptance roadmap](managed-video-roadmap.md) and the
[specification](spec.md#v13-managed-video-implementation-boundary). Release
acceptance remains separate from implementation and local test success.

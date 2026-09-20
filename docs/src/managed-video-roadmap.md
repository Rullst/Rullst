# Managed private video candidate

On September 20, 2026 the owner asked about Bunny.net video hosting and a native
Rullst integration. No Bunny adapter, provider-aware player or management API is
implemented in the current repository. Existing LMS media fields and HTML media
elements do not provide remote upload, transcoding or private playback control.

A Bunny Stream integration is selected for implementation after the deployment
diagnostic and supervision observation base. The owner's subsequent September
20 direction prioritizes completing the supported private-video lifecycle before
starting Labs or another optional integration. This is a delivery target, not
implemented functionality or a promise that every Bunny service/API belongs in
v13. Acceptance by scope freeze remains required; the September 24–26
validation/publication window remains reserved.

## Proposed first journey

1. An authorized instructor creates a tenant/course-bound asset record. Resolve
   ownership on the server; caller-supplied library/video IDs cannot authorize
   upload, playback, modification or deletion.
2. Grant one bounded upload to the provider without exposing account/library
   management keys to the browser. Review TUS authorization, limits, expiry,
   retries and uncertain creation results before implementing a public API.
3. Track processing through authenticated notifications and authoritative
   provider reads. Verify exact raw-body signatures in constant time. The current
   Bunny webhook v1 signature does not bind a timestamp: duplicate/reordered or
   replayed notifications cannot independently authorize publication or regress
   current state.
4. Check current membership and course entitlement before issuing a short-lived
   playback token. Permission withdrawal stops new grants; a previously issued
   provider bearer token may remain usable until expiry. Do not advertise it as
   inherently bound to a Rullst login session or immediately revocable.
5. Exercise a real HTTP/browser consumer, unavailable/deleted assets, expiry,
   tenant/owner denial and replay/failure cases. Keep captions/transcripts and
   keyboard accessibility explicit. A mock or signed URL unit test alone cannot
   establish real provider playback acceptance.

## Completion target before another optional integration

The first release target is a usable private course-video integration, with
the complete supported lifecycle and its recovery paths:

- Persist tenant/course ownership, provider asset identity and versioned local
  lifecycle state. Provide authorized creation, bounded listing/status reads,
  metadata updates, publication/withdrawal and deletion with reconciliation.
- Authorize resumable uploads without browser access to library management
  secrets; handle expiry, retry, cancellation and ambiguous remote outcomes.
  Record the provider limits actually enforced instead of promising unsupported
  restrictions. Reconcile creation that may already have succeeded remotely
  before retrying it.
- Process authenticated notifications with durable deduplication and explicit
  authoritative refresh. Cover duplicates, replay, reordering, failed processing
  and missed notifications without silently publishing or regressing assets.
- Issue and renew short-lived playback grants only after current entitlement
  and asset-state checks. Document required embed and direct-file/CDN settings,
  expiry after withdrawal, unavailable/deleted assets and key/configuration
  changes; do not claim instantaneous invalidation of issued bearer tokens.
- Exercise the lifecycle in a focused authenticated application/browser
  consumer. Include visible processing/failure states, keyboard-accessible
  playback controls and a documented captions/transcripts integration path.
- Deliver typed errors, bounded request/response/deadline behavior, redacted
  diagnostics, deterministic offline mode, setup and operations documentation,
  and packaged-consumer tests. Production must explicitly reject mock grants.

The implementation must first settle and record its SST/storage/API decisions.
Automated HTTP fixtures must test the selected provider protocol and failure
paths; browser tests must exercise the actual application authorization and
player lifecycle with controlled media. Those tests cannot establish real Bunny
transcoding or playback compatibility. The owner's no-live-account-testing
direction below remains in force and must appear in the delivered status.

Paid DRM, additional video providers and other Bunny products remain separate
scope. They do not prevent completion of this private-course-video profile.
Conversely, a token helper and mocked success response alone do not complete it.

## Architecture and provider boundary

Keep video optional and separate from Core's default runtime. Evaluate a focused
media crate against existing storage responsibilities before adding a package;
no crate name or public provider API is committed by this roadmap. Reuse Auth,
tenant context and existing application entitlements through explicit composition.
Start with one provider and support declared capabilities instead of pretending
all video services have interchangeable APIs.

Embed-view tokens protect the provider player entry point. CDN/file tokens,
direct playback/original-file exposure, allowed domains, CSP/COEP and DRM are
distinct controls and must be reviewed together. Domain restrictions are not
student authentication. No integration can promise prevention of every copy,
recording or redistribution. Enterprise DRM needs its own provider configuration,
fees and device/license acceptance; it is not implied by ordinary signed URLs.

The owner confirmed an existing Bunny account and Stream library on September
20, then explicitly requested no manual or real-provider testing during this
implementation window. Do not use that account/library, create paid resources,
upload user media or change live settings. Continue automated tests with local
protocol fixtures, disposable infrastructure and simulated provider failures.
Empty/mock credentials need deterministic offline behavior; production cannot
use that mock as playback authorization. Document real-provider interoperability
as unvalidated and keep the initial integration opt-in with a precise supported
scope. Later application fixes should be contributed back to the framework.

## Primary references reviewed on September 20

- [Bunny Stream pricing](https://bunny.net/docs/stream/pricing): storage counts
  generated renditions and optional retained files; delivery tier and viewer
  region affect the bill. Basic token integration does not include paid DRM.
- [Embed view authentication](https://bunny.net/docs/stream/token-authentication):
  server-generated video/expiry signatures and separately enabled library policy.
- [Signed processing webhooks](https://bunny.net/docs/stream/webhooks): exact body,
  version/algorithm headers and library read-only API key; no signed timestamp.

Pricing and API details must be checked again during adapter implementation.
This proposal is not a claim that Bunny is universally cheapest or that the
current Rullst release already supports it.

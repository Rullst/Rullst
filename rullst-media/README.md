# rullst-media

v13 candidate for private course videos with Bunny Stream, first published in
the 13.0.0-alpha.1 prerelease. The crate
provides creation, metadata, resumable upload authorization, processing refresh,
explicit publication, current-entitlement playback, withdrawal, deletion and
recovery of interrupted operations. An experimental `s3` adapter stores and
serves the original upload in a private S3, R2 or MinIO bucket instead, without
transcoding. It is **opt-in**, with its standalone package
now included in the v13 release inventory. Final release acceptance remains
outstanding.
A production application uses the Bunny adapter with a real Bunny Stream library
(create, browser upload, status, update, deletion and private playback). Rullst's own
automated acceptance uses local HTTP protocol fixtures and controlled browser media,
so transcoding, CDN and player edge cases are not covered by it.
The `s3` adapter is validated against the offline mock only.

**Maturity: Experimental** ([tiers](https://rullst.github.io/Rullst/book/maturity.html)):
the API may change between 13.x releases and not every real-world scenario is
validated yet. Experimental does not mean broken.

[PR #227](https://github.com/Rullst/Rullst/pull/227) passed hosted workspace,
platform and installed-archive source admission at `b21d52e7`. The final release
campaign and provider interoperability remain separate requirements.

## Features and executable example

| Features | Available surface |
| --- | --- |
| none (default) | Validated identities/metadata, authorization and provider contracts; no HTTP or database |
| `bunny` | Bunny Stream adapter, signatures, bounded HTTP and browser upload module |
| `sqlite` | Shared-local durable assets, leased operations and application service |
| `bunny,sqlite` | Complete supported private-video service composition |
| `s3` | Experimental S3-compatible object storage (AWS S3, Cloudflare R2, MinIO): presigned upload/playback of the original, no transcoding |
| `s3,sqlite` | Object-storage service composition; see [the guide](https://rullst.github.io/Rullst/book/managed-video.html#s3-compatible-object-storage-s3-r2-minio) |

Run `cargo run -p rullst-media --example private_course_video --all-features`
from a clone of the repository. [The example](https://github.com/Rullst/Rullst/blob/main/rullst-media/examples/private_course_video.rs) uses a real
temporary SQLite database and an explicit offline provider. It creates, publishes,
checks entitlement, withdraws and deletes a lesson, without network credentials.
Its mock grants are deliberately unusable for browser playback/upload.

There is no default Core/Auth/ORM dependency, framework facade feature or automatic
blueprint modification. Applications opt in explicitly with
`rullst-media = { version = "=13.0.0-alpha.4", features = ["bunny", "sqlite"] }`.
`13.0.0-alpha.4` is a prerelease, not a stable crates.io release.

## Server composition

1. Construct `BunnyCredentials` with **four separate purposes**: library write API
   key, library read-only key for webhook HMAC, embed token key and CDN token key.
   Read secrets through your server's secret configuration; never send them to a
   browser or log configuration. All-empty or `mock_*` credentials mean `Offline`;
   mixed modes fail. `fixture_*` requires the explicitly local test constructor.
2. Construct `BunnyConfig` with `LibraryId`, an environment `Reference`, the
   assigned canonical `*.b-cdn.net` host and `PrivateDelivery::configured(...)`.
   All three protection assertions must be true. This is an **operator assertion**,
   not account discovery: enable embed authentication, advanced CDN token checks
   and protected direct files in Bunny. Review original files, alternate renditions,
   thumbnails/previews, allowed domains and CORS separately. Custom CDN domains
   and automatic library administration are outside this profile.
3. Create `BunnyStream::new(config)`. It uses the fixed Bunny API origin, no ambient
   proxy and no redirects. `StoreConfig::production(provider.binding(), capacity)`
   rejects offline/fixture mode. Initialize a **new** file once using
   `SqliteMedia::initialize`; on restart use `SqliteMedia::open` with the same
   binding/capacity. Production remote mode remains named `RemoteUnvalidated`.
4. Supply trusted UTC through `SystemClock` or a checked `Clock` implementation.
   Compose the provider and store with `MediaService::new`. Implement
   `Authorization::check` by reading authoritative authenticated membership,
   role and current course entitlement. `Manage` controls inventory and mutations;
   `Play` grants playback only. `Permission::until` caps capability lifetime.
   A browser-provided teacher flag, tenant or provider video ID is not authority.
5. Resolve `Scope`, actor and local asset ID under that policy on **every** route.
   Require authentication, CSRF, secure headers, WAF and ownership checks. Bind
   expected revisions for mutations; return minimized errors, not provider bodies.
   The full authenticated Core composition is exercised in
   [the browser integration test](https://github.com/Rullst/Rullst/blob/main/rullst-media/tests/browser.rs).

### Upload-only configuration and direct adapter use (13.0.0-alpha.5)

An application that signs its own player links, or delivers no video through
this crate, can manage videos with the library API key alone:
`BunnyCredentials::upload_only(api_key)` and
`BunnyConfig::upload_only(library, environment, credentials)` need no webhook,
embed or CDN key, no CDN host and no `PrivateDelivery` assertion.
`BunnyStream` then creates, signs upload grants, reads, lists, renames and
deletes videos; `playback` and `verify_notification` return
`MediaError::Unsupported`. `BunnyConfig::new` refuses upload-only credentials.
The provider binding of an upload-only configuration differs from a complete
one's, so a `sqlite` store created with one refuses the other.

Code that calls the `VideoProvider` directly, without `MediaService`, can use
`create_with_metadata(marker, &metadata)`: one call that creates the video and
applies its title and description. Bunny's create request takes only a title,
so the adapter still creates it with the random marker title and then updates
it (two requests): the marker stays the identity that `find_created` searches
after an ambiguous creation. If the metadata step fails, the created video is
returned with its marker title; retry `update` with its id.

### Conformance kit (13.0.0-alpha.5)

The `conformance` feature adds `conformance::check_video_provider`, the
contract the offline Bunny adapter passes, for an application's own
`VideoProvider` or a wrapper around a built-in one. `VideoExpectations::new(now)`
expects upload grants, playback grants and listing; `without_playback()` (as
with upload-only credentials), `without_upload()` and `without_listing()`
require `MediaError::Unsupported` instead. The kit checks marker validation,
the create/get/update/delete lifecycle, grant expiry, mode, HTTPS and TTL
bounds, bounded listing pages and offline determinism, and returns a
`ConformanceReport`. Run it against the offline mode or a disposable library:
it creates and deletes videos. An application-owned adapter creates its own
grants with `UploadGrant::new(&binding, video, protocol, endpoint, signature,
expires_at)` and `PlaybackGrant::new(&binding, url, expires_at)`, which take
the binding's library and mode and refuse non-HTTPS or credential-bearing URLs,
empty or non-ASCII signatures and impossible expiry times with `InvalidInput`;
`UploadGrant::with_declaration` adds a declared type and length. Its `list`
builds pages with `ListedVideo::new(id, title, length_seconds, processing)`
(optional `with_created_at` and HTTPS `with_thumbnail_url`) and
`VideoPage::new(&query, items, total)`, which echoes the query and refuses
more than `per_page` items. See
[Provider conformance kits](https://github.com/Rullst/Rullst/blob/main/docs/src/provider-conformance.md).

### Library listing (13.0.0-alpha.5)

`VideoProvider::list(&ListQuery)` returns one `VideoPage` of the provider
library, including videos uploaded through the Bunny dashboard or before the
application existed. `ListQuery::new(page, per_page)` takes a page from 1 and
10 to 100 videos per page; `with_search(term)` keeps titles containing the term
(case-insensitive, at most 100 bytes, blank clears it) and
`with_order(ListOrder::Title)` sorts by title instead of upload date. Each
`ListedVideo` holds the id, title, duration, `Processing` state, upload time in
UTC seconds and the provider's `https` thumbnail location; `VideoPage` holds the
page, page size, total matches and `has_more()`. The adapter sends
`GET /library/{id}/videos?page=&itemsPerPage=&orderBy=&search=` with the
library key, the same timeouts, no redirects, one bounded GET retry and a 1 MiB
response cap, and rejects a page whose library, page, size, status or dates
violate the contract (`Protocol`). Offline (`mock_*`) mode lists a fixed sample
library of three videos (ready, processing, failed) that `get` does not store.
Listing does not grant ownership: the application decides who may see the
library and which listed video becomes one of its assets. A thumbnail URL is a
location, not a playback grant; under CDN token authentication it needs a
signature. The `s3` adapter and other implementations return `Unsupported`
unless they override `list`.

`create` uses a local creation ID bound to the original actor and metadata. Retain
that ID for retries. `get` and cursor-based `list` require management permission;
listing is limited to the authorized course and 100 items. `Asset.pending`,
`lifecycle`, `processing`, `published` and `revision` describe distinct states.
Metadata is plain text, with a 200-byte title and 4096-byte description.
Updates preserve unrelated provider meta tags from a bounded current read. A
full list without room for the description fails rather than dropping a tag.
Bunny's update model has no description property, so the adapter writes the
description as the `description` meta tag and verifies that same tag in the
authoritative read. API 1.6.6 documents that this tag also updates the separate
top-level Description, which Smart Generate may rewrite; that field is never
compared, so a generated summary cannot leave an update unverifiable.
The host must coordinate other tools writing the same remote video: Bunny does
not supply a conditional-update token for atomic conflict detection between them.

## Upload, processing and playback

Serve `BUNNY_UPLOAD_MODULE` as a same-origin external JavaScript module with the
correct MIME type. Instantiate `BunnyUpload` with the chosen `File`, title,
`getGrant({signal})` and progress callback. The callback calls your authenticated,
CSRF-protected upload-grant route; serialize the returned `UploadGrant` only to
that authorized client. `start()`, `pause()` and `cancel()` are explicit. Resume
uses the **same instance/File** and HEAD-confirmed server offset; no file or token
is put in localStorage. Reload/reselect fingerprint recovery is not implemented.

The browser defaults to at most 1 GiB, accepted video MIME types, 1 MiB chunks,
4096 HTTP requests, 30-second request/authorization waits and a 15-minute run.
The host may lower the byte limit or, since 13.0.0-alpha.5, raise it with
`maxBytes` up to `BunnyUpload.MAX_BYTES` (64 GiB). Files above 2 GiB use larger
whole-MiB chunks (at most 2048 of them, 32 MiB at 64 GiB) and files above
900 MiB a longer run limit of one second per MiB; `pause()`/`start()` still
resume from the server offset. `types` replaces the accepted media types
(`BunnyUpload.DEFAULT_TYPES` is `video/mp4`, `video/webm`, `video/quicktime`)
with 1 to 16 lower-case `video/*` types. It checks renewal identity/origin and omits
cookies and library keys from provider requests. The server caps upload grants
at 3600 seconds and current permission expiry; the module refuses a grant that
expires more than 3600 seconds plus a 300-second clock-skew allowance after its
own clock. These client limits are **not
cryptographic provider quotas**: a copied TUS bearer may be reusable until expiry.
Configure library quotas; cancellation stops local transfer and needs a separate
authorized deletion request to remove the remote asset. Ambiguous TUS creation
is reported explicitly and is never blindly retried as another POST.

`upload` refreshes remote state before issuing a grant. Processing completion
is independent of TUS byte acceptance. Show processing/failure state, allow
`refresh` for missed notifications, and require the instructor's explicit
`publish` after the API reports Ready. Completion webhooks never publish.

`playback` checks current entitlement, reads current provider readiness and
fences local withdrawal/deletion in the transaction that issues a grant bounded
by permission expiry and at most 900 seconds. It is a read, not a leased
mutation: concurrent viewers never wait for one another's provider reads or
leases, and a failed, timed-out or dropped request leaves no durable intent. It records a changed observation
only when the revision is unchanged and no lease is live; a video that is no
longer ready withdraws publication. Rate-limit the playback route per actor.
Supported kinds are `Embed`, directory-protected `Hls`, and `Mp4_720p` only when
the current API reports both MP4 fallback and that resolution. Return the grant
using a private `Cache-Control: no-store` response and `Referrer-Policy: no-referrer`;
never log its URL. Renew through the same authenticated endpoint before expiry.
The host must handle an expired player gracefully and choose a TTL appropriate
to its privacy/access policy. Do not rewrite HLS segments outside their signed
directory or assume an embed token protects direct files.

`withdraw` immediately blocks new grants, even with provider work pending;
`delete` records denial before remote deletion, then confirms absence via GET.
Existing provider bearer URLs, an already initialized player and cached media
may survive local withdrawal/deletion. This is not session-bound DRM or a
promise of instant remote revocation, copy prevention or cache/backup erasure.

For embed playback allow only the configured player in CSP `frame-src`; allow
Bunny upload in `connect-src`. For direct playback configure `media-src` and
manifest/segment CORS. The host must review COEP compatibility with the provider;
change it only on the reviewed player page, with the rest of the secure baseline
intact. Supply captions through the provider's configured tracks or an authorized
WebVTT route for a native player, a visible transcript, titled iframe and keyboard
controls. Caption management API and automatic transcription are not included.
The browser acceptance decodes controlled video and exercises a captions track
and keyboard playback; it does not claim Bunny iframe accessibility conformance.

## Signed notifications and recovery

Accept a bounded exact raw body (4096 bytes maximum), before JSON conversion.
Require exactly one each of Bunny's signature version, algorithm and signature
headers at ingress; call `verify_notification` with those values. The adapter verifies
v1/HMAC-SHA256 using the **read-only library key**, constant-time comparison,
library identity and documented status range. Pass only `VerifiedNotification`
to `MediaService::notification`. Use Core's exact `MachineEndpoint` signature
verification boundary if exempting this route from cookie CSRF; do not exempt an
entire path prefix. Apply ingress rate limits and minimized logging.

The signed notification has no timestamp. It is a refresh hint for an already
owned video, never an authority to create ownership, publish, grant access or
replay old processing status. The store retains at most 32 digests per asset
and spaces successful refreshes by at least two seconds. Older replays outside
this bound may cause another authoritative GET; this is not an unlimited replay
ledger. API and webhook numeric status mappings differ and are handled separately.

| Result/state | Required handling |
| --- | --- |
| `Conflict` | Reload current revision/state before a deliberate new action |
| `Busy` | Respect the in-flight lease or local store contention; retry with bounded backoff |
| `Unavailable`, `Uncertain`, timeout or process death | Inspect persisted intent; `reconcile` after its 45-second lease, rechecking management authorization |
| Unknown create result | Search the persisted opaque marker; zero or multiple matches stop the intent as `CreationUnconfirmed`; never generate another create request automatically |
| `Asset.failure` set (stopped create/update) | Never retried automatically; mutations, `reconcile` and upload return `Conflict` while withdrawal and playback still work. Fix the cause, then call `retry_failed` or `discard_failed` with the current revision |
| Pending refresh | Upload or the same notification can resume it after lease expiry; they cannot take over pending update/delete/create. Playback neither takes nor waits for leases |
| `Denied`, `Expired` | Reauthenticate/recheck entitlement; issue no capability |
| `Protocol`, `Configuration`, `Storage`, clock rollback | Stop granting access; investigate rather than reset/repair the store |
| `Capacity` | Review retention/capacity without deleting active or uncertain operations |

Mutations are journaled before dispatch; no DB write lock is held during HTTP.
Every operation, reads and playback included, runs short serialized
`BEGIN IMMEDIATE` transactions on a pool of four connections. A read commits
no change unless the stored clock high-water mark advances, so a burst within
one second adds no durable write. A SQLite lock or connection wait over 5
seconds returns `Busy`, not `Storage`: nothing was committed, so retry.
Remote metadata updates and deletions are reconciled using authoritative reads.
Every public operation has a 20-second total timeout; provider calls have bounded
request and response sizes, with at most one GET retry and no automatic mutation
retry. A timed-out future does not prove that the provider rolled back its work.
Remote marker lookup is bounded to 100 results; out-of-band renaming before
identity binding or ambiguous search needs operator investigation.

A create or metadata update that fails for a non-transient reason stops rather
than staying pending forever. The failing call returns the provider error and
`Asset.failure` records a bounded, non-secret `OperationFailure`: `Rejected` (a
definitive refusal; a refused create made no remote video), `CreationUnconfirmed`,
`RemoteMissing`, `TagCapacity` or `VerificationMismatch`. Transient outcomes such
as `Unavailable`, a timeout or a protocol violation keep the intent leased for
`reconcile`. `retry_failed` resends a refused create, only searches the marker
again for an unconfirmed one and re-executes a stopped update. `discard_failed`
turns a stopped create into a local tombstone: retire its creation ID; a remote
video carrying the marker that exists or appears later is neither owned nor
deleted. A discarded update keeps the requested local metadata while the remote
may keep earlier values; the asset can then be refreshed, published or deleted.

## Durable storage and operation

SQLite is shared-local on trusted local storage, not a multi-host/network-FS
backend. Initialization never overwrites an existing file. Open checks exact
schema and persisted library/environment/mode/capacity; indexed identities must
agree with bounded records. Transactions reject clock rollback. The host owns
private directories, filesystem encryption, WAL/backup/restore consistency and
exclusive schema administration. Credentials are not stored in asset records.
Key rotation with unchanged environment/library is accepted; it can invalidate
old capabilities. Changing environment, origin, CDN host, mode or capacity needs
an explicit reviewed migration, not silent opening of unrelated state.

Capacity is store-wide: every tenant and course shares `max_assets`, and
deleted tombstones count until purge. A store shared by several tenants should
add `StoreConfig::tenant_assets(n)` so one tenant's managers cannot exhaust it
for the others; the quota is persisted like capacity. Otherwise use one store
per tenant.

`purge_deleted` removes at most 100 confirmed or discarded local tombstones per
authorized call, with a cutoff at least 24 hours old. It preserves active and
pending assets. Once purged, **retire the creation ID**: its idempotency/replay
memory ends there. Deletion, or discarding a stopped create, clears local
metadata before this retention step and replaces the creation digest, an
unsalted hash of the original title and description, with a fixed tombstone
value. Until purge, `create` with that ID returns `Conflict`.
This is not proof of physical disk, provider backup or CDN erasure. Restore
policy must address stale permissions, keys, retired IDs and provider
reconciliation.

## S3-compatible object storage

The experimental `s3` feature stores and serves the uploaded original in a
private bucket; it never transcodes. Upload grants come from
`MediaService::upload_declared` with an `UploadDeclaration` (content type and
exact length) and are presigned `PUT` requests; `PlaybackKind::Original` grants
are presigned `GET` requests. Both expire within 900 seconds. `refresh` or
`publish` confirms the object with `HEAD` before it becomes ready.
**Status: Experimental; validated against the offline mock only; provider
interoperability not yet validated.** Configuration, bucket CORS and limits are
in the [private video guide](https://rullst.github.io/Rullst/book/managed-video.html#s3-compatible-object-storage-s3-r2-minio).

## Acceptance and limits

Run `cargo test -p rullst-media --all-features`, strict Clippy and the executable
example. `node .github/media-upload-tests.mjs` tests upload failures without a
network. On Linux with Node 24 and Chrome installed,
`RULLST_MEDIA_BROWSER_TESTS=1 cargo test -p rullst-media --all-features --test browser`
exercises actual authenticated HTTP, CSRF/tenant denial, TUS bytes, controlled
media decoding, captions/keyboard access, withdrawal and deletion. CI must also
validate features, extracted package consumers and the workspace gates before
release admission. Browser fixtures use synthetic keys and literal loopback;
production code must not select their constructor or store mode.

Paid DRM, custom domains, caption CRUD, multiple CDN/providers, live account
configuration verification and multi-host storage are outside this supported
profile. API/signature references are Bunny's official
[OpenAPI](https://video.bunnycdn.com/openapi/bunnynet-video-api.public.json),
[TUS guide](https://bunny.net/docs/stream/tus-resumable-uploads),
[embed authentication](https://bunny.net/docs/stream/token-authentication),
[webhooks](https://bunny.net/docs/stream/webhooks) and
[advanced CDN tokens](https://bunny.net/docs/cdn/security/token-authentication/advanced),
reviewed September 20, 2026. Local protocol vectors are not live interoperability evidence.

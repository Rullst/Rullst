# rullst-media

Unpublished v13 candidate for private course videos with Bunny Stream. The crate
provides creation, metadata, resumable upload authorization, processing refresh,
explicit publication, current-entitlement playback, withdrawal, deletion and
recovery of interrupted operations. It is **opt-in and not admitted for release**.
Real Bunny account, transcoding, CDN and player interoperability are **unvalidated**;
automated acceptance uses local HTTP protocol fixtures and controlled browser media.

## Features and executable example

| Features | Available surface |
| --- | --- |
| none (default) | Validated identities/metadata, authorization and provider contracts; no HTTP or database |
| `bunny` | Bunny Stream adapter, signatures, bounded HTTP and browser upload module |
| `sqlite` | Shared-local durable assets, leased operations and application service |
| `bunny,sqlite` | Complete supported private-video service composition |

Run `cargo run -p rullst-media --example private_course_video --all-features`
from this checkout. [The example](examples/private_course_video.rs) uses a real
temporary SQLite database and an explicit offline provider. It creates, publishes,
checks entitlement, withdraws and deletes a lesson, without network credentials.
Its mock grants are deliberately unusable for browser playback/upload.

There is no default Core/Auth/ORM dependency, framework facade feature, automatic
blueprint modification or crates.io installation command for this candidate.
Applications may use a reviewed local path dependency with `bunny` and `sqlite`.

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
   [the browser integration test](tests/browser.rs).

`create` uses a local creation ID bound to the original actor and metadata. Retain
that ID for retries. `get` and cursor-based `list` require management permission;
listing is limited to the authorized course and 100 items. `Asset.pending`,
`lifecycle`, `processing`, `published` and `revision` describe distinct states.
Metadata is plain text, with a 200-byte title and 4096-byte description.
Updates preserve unrelated provider meta tags from a bounded current read. A
full list without room for the description fails rather than dropping a tag.
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
The host may lower the byte limit. It checks renewal identity/origin and omits
cookies and library keys from provider requests. The server caps upload grants
at 3600 seconds and current permission expiry. These client limits are **not
cryptographic provider quotas**: a copied TUS bearer may be reusable until expiry.
Configure library quotas; cancellation stops local transfer and needs a separate
authorized deletion request to remove the remote asset. Ambiguous TUS creation
is reported explicitly and is never blindly retried as another POST.

`upload` refreshes remote state before issuing a grant. Processing completion
is independent of TUS byte acceptance. Show processing/failure state, allow
`refresh` for missed notifications, and require the instructor's explicit
`publish` after the API reports Ready. Completion webhooks never publish.

`playback` checks current entitlement, refreshes provider readiness, fences local
revision and issues a grant bounded by permission expiry and at most 900 seconds.
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
| `Busy` | Respect the in-flight lease; retry with bounded backoff |
| `Unavailable`, `Uncertain`, timeout or process death | Inspect persisted intent; `reconcile` after its 45-second lease, rechecking management authorization |
| Unknown create result | Search the persisted opaque marker; zero or multiple matches remain uncertain; never generate another create request automatically |
| Pending refresh | Upload/playback or the same notification can resume it after lease expiry; they cannot take over pending update/delete/create |
| `Denied`, `Expired` | Reauthenticate/recheck entitlement; issue no capability |
| `Protocol`, `Configuration`, `Storage`, clock rollback | Stop granting access; investigate rather than reset/repair the store |
| `Capacity` | Review retention/capacity without deleting active or uncertain operations |

Mutations are journaled before dispatch; no DB write lock is held during HTTP.
Remote metadata updates and deletions are reconciled using authoritative reads.
Every public operation has a 20-second total timeout; provider calls have bounded
request and response sizes, with at most one GET retry and no automatic mutation
retry. A timed-out future does not prove that the provider rolled back its work.
Remote marker lookup is bounded to 100 results; out-of-band renaming before
identity binding or ambiguous search needs operator investigation.

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

`purge_deleted` removes at most 100 confirmed local tombstones per authorized
call, with a cutoff at least 24 hours old. It preserves active/pending assets.
Once purged, **retire the creation ID**: its idempotency/replay memory ends there.
Deletion clears local metadata before this retention step. This is not proof
of physical disk, provider backup or CDN erasure. Restore policy must address
stale permissions, keys, retired IDs and provider reconciliation.

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

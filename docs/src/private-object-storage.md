# Private object storage

The v13 `storage-s3` candidate adds optional AWS S3 and Cloudflare R2 operations
to `rullst-core::Storage` and the `rullst` facade. It supports bounded object
upload, download, metadata, deletion and signed GET URLs. Bare Core does not
enable this feature or its HTTP/signing dependencies.

Protocol tests and a digest-pinned disposable Garage service exercise signature
rejection and restart persistence. Hosted workspace/platform and extracted-package source admission passed in [PR #236](v13-delivery-plan.md#six-increment-source-admission-on-september-21). Final release admission remains separate.
No AWS/R2 owner account has been exercised; bucket policy and provider
interoperability remain deployment responsibilities.

## Explicit configuration

Enable `storage-s3` in Core or in the facade. To use the authorization helpers
below, also enable the facade's `security` feature.

```toml
[dependencies]
rullst = { version = "13.0.0-alpha.1", default-features = false, features = ["storage-s3", "security"] }
```

The version above identifies the unpublished candidate, not an available
registry release. Use the reviewed candidate source or extracted packages until
registration and release admission complete.

```rust,no_run
# #[cfg(all(feature = "storage-s3", feature = "security"))]
# fn configure() -> Result<(), Box<dyn std::error::Error>> {
use rullst::Storage;
use rullst::storage::cloud::{CloudCredentials, CloudStorageConfig};

let credentials = CloudCredentials::new(
    std::env::var("R2_ACCESS_KEY_ID")?,
    std::env::var("R2_SECRET_ACCESS_KEY")?,
)?;
let config = CloudStorageConfig::new(credentials).require_production()?;
let storage = Storage::r2("private-files", std::env::var("R2_ACCOUNT_ID")?)
    .with_cloud_config(config)?;
# let _ = storage;
# Ok(())
# }
```

For AWS, replace `Storage::r2` with `Storage::s3(bucket, region)`. Attaching the
configuration validates the provider parameters without making a network call.
Credentials are explicit: the library never searches environment variables,
profiles, instance metadata or ambient proxies. Known temporary credentials can
include a session token and expiration through `with_session_token`.

Default limits are **16 MiB per object** and **30 seconds per request**. A bounded
download additionally has a total operation deadline. `with_limits` accepts
1 byte–256 MiB and 1 millisecond–120 seconds. Requests use HTTPS, reject
redirects and disable transparent content decompression. The caller owns
concurrency limits; the per-object ceiling is not a process-wide memory quota.
Uploads are buffered and sent once. Errors do not trigger automatic retries or
imply that a timed-out write was never committed; reconcile state before retrying
an ambiguous write, especially with provider versioning enabled.

## Authorize before accessing or sharing

Load account membership and file ownership from authenticated, authoritative
application state. Check the tenant boundary and then the owner/role policy
before obtaining `TenantStorage` or issuing a signed URL. A tenant prefix does
not authorize every member to access every object in that tenant.

`TenantStorage` applies `tenants/<tenant>/` to upload, download, metadata,
deletion and download grants. Cloud keys reject empty/parent/dot components,
backslashes, control characters and more than 1,024 UTF-8 bytes. Filenames with
spaces, Unicode and literal percent signs are encoded as object keys. The
[archive consumer](../../.github/fixtures/storage-facade.rs) demonstrates
owner/tenant enforcement, including denial of an administrator from another
tenant, using the same facade API shipped in the archive.

Use the existing upload admission/quarantine contract before persisting or
sharing untrusted files. The storage transport is not a malware scanner. Uploads
use `application/octet-stream`; custom browser rendering and media transforms
are outside this increment.

## Private downloads and deletion

`signed_download(key, lifetime)` issues a GET grant valid for **1–900 whole
seconds**, bounded by any known credential expiration. It does not query object
existence. `SignedDownload::expose_url()` deliberately exposes the bearer secret;
its Debug output is redacted. Keep that value out of logs, analytics and public
pages. The configured private backend rejects the older unsigned `url()` helper.

Anyone holding a valid URL can reuse it until expiry or provider invalidation.
It is not single-use, does not recheck the application's current permissions on
each download and cannot promise individual immediate revocation. Use an
authenticated application download endpoint when every read must consult current
access policy. `expires_at()` describes the requested deadline; the provider
can invalidate access earlier.

`metadata` returns the byte length and an optional opaque ETag. ETags are not a
portable content hash. Remote `delete` is idempotent for a missing key; a
versioned bucket may retain older object versions according to provider policy.
The adapter does not administer lifecycle rules, backups, bucket ACLs or IAM.

## Deterministic offline mode and validation

Two empty credential values or two `mock_*` values select an in-memory store,
shared by clones of the configured `Storage`. Mixed live/mock or partially empty
credentials fail. Offline capacity is 256 objects and 64 MiB total, in addition
to the configured per-object limit. It cannot issue a real provider URL.
`require_production()` rejects this fallback and development endpoints.

`with_loopback_test_endpoint` accepts only literal-loopback HTTP(S) endpoints
without user information, path prefixes, query strings or fragments. It cannot
modify a configuration already checked for production.

The automated checks cover tenant/owner denial, canonical keys, credential and
grant expiry, HTTP failures, declared/chunked response bounds and timeouts. The
independent S3 fixture runs with an immutable image digest, loopback-only port,
resource limits and an owned disposable data directory. It rejects unsigned,
altered and expired URLs and verifies data after service restart. This verifies
the implemented S3 protocol journey; it does not establish AWS/R2 account
configuration, all S3 extensions or multi-region failover.

Direct presigned uploads, multipart streaming, image processing and additional
S3 operations remain future increments.

# Resumable private multipart uploads

The unpublished v13 candidate adds Core/facade `storage-multipart`. It extends
the existing private S3/R2 adapter with server-mediated parts and recovery after
an interrupted connection or process restart. It does not change default features.
The application sends bounded parts through its authenticated server; provider
credentials remain there. Direct browser presigned uploads are outside this API.

## Bind the object and policy

Resolve the current account, tenant membership and object permission before each
operation. `TenantStorage::multipart` confines the object to that authenticated
tenant. Raw `Storage::multipart` is an operator-level API for an already-approved
complete object key. A checkpoint does not authenticate its presenter.

```rust,no_run
# #[cfg(feature = "storage-multipart")]
# mod example {
use rullst::{TenantStorage, storage::cloud::multipart::{
    MultipartKey, MultipartLimits, MultipartStorage,
}};
use std::time::Duration;

fn uploader(tenant: &TenantStorage, independent_key: String)
    -> Result<MultipartStorage, Box<dyn std::error::Error>>
{
    let key = MultipartKey::new(independent_key)?;
    let limits = MultipartLimits::new(
        1024 * 1024 * 1024, 8 * 1024 * 1024, Duration::from_secs(86400),
    )?;
    Ok(tenant.multipart("quarantine/server-allocated-unique-name", key, limits)?)
}
# }
```

Use the existing explicit `CloudStorageConfig` and its `require_production`
guard at startup. The checkpoint key is a separate 32-byte random secret encoded
as canonical unpadded base64url; provision it through your secret store. All
instances resuming the same uploads need the same key and exact configuration.
Rotation requires retaining the previous key/configuration for its existing
uploads until completion or abort. Debug/error output omits keys and checkpoints.

Limits fix a maximum total length, one part size from 5 to 64 MiB, at most 256
parts and an exact lifetime from one minute through seven days. Total capacity
therefore cannot exceed 16 GiB. Each `begin(total_bytes)` fixes the actual length;
all non-final parts have the configured size and the last has the exact remainder.
An application must separately bound concurrent requests, active uploads and
tenant quotas. Ordinary `get` retains its existing configured download limit;
use an authorized signed GET for objects larger than that limit.

## Persist and resume

1. Call `begin` for a fresh, server-allocated private/quarantine object name.
2. Persist the checkpoint through `expose_encoded()` in an authenticated, durable
   application record. Restore with `MultipartCheckpoint::from_encoded`.
3. Call `upload_part` with the checkpoint, one-based number, exact bytes and
   expected SHA-256. Persist the returned replacement checkpoint atomically.
4. After restart, recreate the same uploader and call `progress`. Every expected
   part is reported; `matches_checkpoint` is true only when remote length and
   ETag match a receipt authenticated in the checkpoint. Re-upload absent or
   unmatched parts explicitly, including a part whose response was lost.
5. Call `complete` once all receipts are present. Retain the checkpoint until
   completion is acknowledged or explicitly reconciled.

Serialize or compare-and-swap checkpoint updates in the application. Concurrent
workers starting from the same old checkpoint can otherwise lose each other's
receipt updates; `progress` detects this, but does not merge unknown receipts.
The backend retains uploaded bytes; checkpoints retain authenticated metadata,
not file content. Provider lifecycle rules must leave enough time to resume.

Checkpoints use versioned AES-256-GCM with a fresh nonce, authenticated before
JSON decoding. Encoded checkpoints are capped at 256 KiB. The associated data
binds endpoint, bucket, region, mock/live profile, exact object and policy, plus
the ephemeral store instance in mock mode. Changing a tenant, object, endpoint, policy,
key or ciphertext denies the operation. The host maintains a disciplined clock;
expiry does not substitute for current membership/object authorization or a
durable application revocation record.

## Integrity and uncertain outcomes

Part bytes must match the supplied SHA-256 before any upload request. SigV4 signs
the exact payload digest; ETags remain opaque provider receipts and are not
treated as a cryptographic content digest. This is transport/content consistency,
not a malware scan or proof that the client supplied safe content. Complete-file
scanning and release from quarantine remain explicit application steps.

Completion sends all consecutive ETags in order and parses the bounded response.
An HTTP 200 containing an XML error never counts as completion; S3 can return
errors after sending its success status. See the
[AWS completion contract](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CompleteMultipartUpload.html).

`CompletionUncertain` means the sender cannot establish the outcome. Do not
automatically initiate another upload or publish the object. Call
`reconcile_completion`: `Confirmed` requires both the expected length and the
random upload marker attached during initiation. Object existence alone is
insufficient. This observation does not prove a complete-file digest, scanning,
continued object immutability or authorization. A same-name upload can replace
an existing object as with ordinary `put`; allocate fresh names and control reads
until the application has accepted the content.

## Abort and abandoned uploads

`abort` accepts authentic expired checkpoints so cleanup can continue after the
upload window closes. It sends abort and checks whether the provider still reports
the upload. `RetryRequired` or an error means retain the cleanup record, stop part
writers and try again. `Gone` refers to the multipart session; abort never deletes
an already-completed object. Concurrent writes can require repeated aborts, as
described by the [AWS abort contract](https://docs.aws.amazon.com/AmazonS3/latest/API/API_AbortMultipartUpload.html).

Schedule cleanup from durable application records and provision provider lifecycle
expiration for abandoned multipart uploads. A lost initiation response may leave
an upload whose identifier never reached the application; the checkpoint API
cannot clean that up. Bucket provisioning and a global orphan sweeper are not
automatic. R2 supports a subset of S3 features; consult the
[provider compatibility table](https://developers.cloudflare.com/r2/api/s3/api/)
for account/deployment requirements.

## Acceptance and operational boundaries

Requests use the existing explicit HTTPS endpoint rules, no ambient proxy, no
redirects and credential-expiry checks. Responses are capped at 256 KiB and
4,096 XML nodes; DTDs, excessive depth, foreign resource identities and ambiguous
or truncated part lists fail closed. At most one 64 MiB part is supplied per call,
with a bounded request copy; application concurrency multiplies that memory cost.

Empty or `mock_*` provider credentials select the bounded offline store. It has
at most 256 active upload/object entries and a shared 64 MiB content budget. It
does not survive process restart and cannot pass `require_production`.

Local Core regression and owned HTTP failure tests passed. The independent,
digest-pinned S3 service passed both endpoint profiles, tenant isolation,
authenticated part upload/list/complete/abort, new client instances, service and
process restart, and completion reconciliation. Restart uses a fixed loopback
port so the approved endpoint remains identical. No owner account was used.
The archive-only facade consumer also passed the ordinary failure contracts and
the native process/service restart journey. Seventeen archives were audited and
all 135 Core source files matched the extracted bytes. Strict all-target Clippy
passed; the standalone production feature graph contains no SQL backend or ORM.
Full hosted workspace, coverage, security and source admission remain required;
no v13 publication is implied.

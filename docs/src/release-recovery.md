# v12 partial-publication and recovery runbook

This runbook covers an interrupted multi-crate publication. A crates.io upload
is irreversible: a version can be yanked but cannot be deleted or replaced with
different bytes. Recovery therefore resumes forward from verified artifacts; it
never tries to overwrite an accepted version.

## Preconditions

- Freeze merges and deployments that consume the affected prerelease.
- Preserve the release workflow run, tag, commit, §checksums.txt§, attestation,
  SBOM, audit output and every §.crate§ archive.
- Revoke a registry token immediately if credential compromise is suspected.
- Assign an incident owner and record times, commands, registry responses and
  affected package versions.

## Name ownership and first-publication credentials

The machine-readable policy in `.github/crates-ownership-policy.json` separates
the expected crates.io owner (`venelouis`) from the GitHub identity trusted to
run the release (`Rullst/Rullst`). The bootstrap allowlist is not an ownership
claim: it is the reviewed set of names that may still return `404` before their
first publication.

`.github/check-crates-ownership.sh` runs once in the verification job and again
immediately before publication. Every registered package must include
`venelouis` among its owners. Every unregistered package must be present in the
bootstrap allowlist. A name registered by another owner, an unexpected missing
name, an API error, or a malformed policy stops the release.

Before the first v12 RC:

1. Protect the GitHub `crates-io` environment with required review and tag
   deployment rules.
2. Configure Trusted Publishing for every already-registered package using
   GitHub owner `Rullst`, repository `Rullst`, workflow `release.yml`, and
   environment `crates-io`.
3. Create a shortest-lived crates.io token with the `publish-new` endpoint
   scope. Restrict its crate-name scope to the reviewed bootstrap names if the
   crates.io UI permits that combination. It does not need `publish-update`:
   registered packages are published with the OIDC credential.
4. Store it only as the `CRATES_IO_BOOTSTRAP_TOKEN` secret in the protected
   `crates-io` environment. Never put it in repository secrets, logs, command
   history, documentation, or a local Cargo credentials file.
5. Start the tag workflow only after reviewing the generated ownership evidence.
   The publish loop selects the bootstrap token only for packages classified as
   `unregistered-reviewed-bootstrap`; registered packages use the short-lived
   Trusted Publishing token.
6. After all new packages are indexed, configure the same Trusted Publisher for
   each of them, enable crates.io's trusted-publishing-only protection, revoke
   the bootstrap token, delete the GitHub environment secret, and remove the
   names from the bootstrap allowlist in a reviewed change.

Trusted Publishing cannot be configured before a crate's first release. The
one-time token is therefore an explicit, bounded exception, not a permanent
fallback. See the official [crates.io Trusted Publishing announcement](https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/)
and [Cargo publishing rules](https://doc.rust-lang.org/cargo/reference/publishing.html).

## Determine the exact state

For each package in §.github/release-order.json§:

1. query §https://crates.io/api/v1/crates/<crate>/<version>§;
2. classify it as not indexed, indexed with the expected checksum, or indexed
   with an unexpected checksum;
3. compare the registry checksum to the retained verified archive;
4. verify that every already-published internal dependency precedes its
   dependent in the release order.

Do not infer success only from the exit status of §cargo publish§. Registry
indexing is asynchronous, so poll with a bounded timeout. The release workflow
performs this check after each package.

## Recovery decisions

### Nothing was accepted

Correct the failure on a new commit. If the tag or package content must change,
use a new prerelease version such as §12.0.0-rc.2§. Never move a published or
publicly attested tag to unrelated content.

### A prefix was accepted with matching checksums

Keep the accepted packages. Re-run the release workflow from the same immutable
tag and verified artifact set. Its resume logic skips only versions whose
crates.io checksum equals the retained archive and continues at the first
missing package.

If source, manifest or artifact content needs any change, bump **all**
publishable packages and internal requirements atomically to a new prerelease.
Do not mix rebuilt bytes into the interrupted version.

### A checksum is unexpected

Stop immediately. Do not publish dependents. Preserve evidence, revoke
credentials, contact crates.io support and treat the event as a potential
supply-chain incident. Yank the affected version when that reduces user risk,
but remember that yanking does not remove downloaded bytes.

### A dependency never becomes index-visible

Stop before its dependents. Retain the successful upload response and poll the
API/index within the bounded operational window. If crates.io reports an
incident, wait for service recovery; do not reorder the DAG or weaken checksum
verification.

## Consumer mitigation

- Yank a broken or unsafe prerelease and publish a corrected new prerelease.
- State exactly which packages/versions were accepted, which were yanked and
  which replacement users should select.
- Never describe yanking as deletion or as proof that no consumer downloaded
  the artifact.
- For a stable vulnerability, follow the advisory SLA and coordinated
  disclosure policy in [Security advisory exceptions](security-advisory-exceptions.md).

## Completion evidence

Recovery is complete only when:

- all 15 expected versions are indexed with retained checksums;
- a clean consumer resolves only registry packages and compiles;
- documentation/index pages are reachable;
- the incident timeline and any yanks/replacements are recorded;
- temporary credentials are revoked and Trusted Publishing is restored;
- the next release includes a regression for the initiating failure.

This process is package recovery, not application rollback. Database migrations,
traffic rollback, secrets and deployed binaries require a separate
application-specific operational plan.

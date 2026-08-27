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

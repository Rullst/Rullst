# Partial-publication and recovery runbook

This runbook covers an interrupted multi-crate publication. A crates.io upload
is irreversible: a version can be yanked but cannot be deleted or replaced with
different bytes. Recovery therefore resumes forward from verified artifacts; it
never tries to overwrite an accepted version.

## Preconditions

- Freeze merges and deployments that consume the affected prerelease.
- Preserve the release workflow run, tag, commit, `checksums.txt`, attestation,
  SBOM, audit output and every `.crate` archive.
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

The one-time bootstrap procedure below was completed before the first v12 RC
and for the four new v13 names in 13.0.0-alpha.1 (see below). For
13.0.0-alpha.4 it is again the reviewed path for one name,
`cargo-rullst-extras` (see
[its section](#first-publication-of-cargo-rullst-extras-1300-alpha4)).
Every registered name still publishes only with Trusted Publishing. The token
path applies only to a name that crates.io does not know and that the tagged
source lists in the bootstrap allowlist:

- `.github/release-preflight.py` and the release verification job accept such
  a name and report it as a first publication that needs
  `CRATES_IO_BOOTSTRAP_TOKEN` in the `crates-io` environment. Any other
  unregistered name, a name registered by another owner and a registry error
  are refused, as before.
- The publish job requires the secret only while such a name exists, and the
  publish loop hands it only to that name's `cargo publish`.

Adding a name to the allowlist is itself a reviewed change. With an empty
allowlist, every unregistered name is refused before the admission.

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
   the bootstrap token, delete the GitHub environment secret, and, in a
   reviewed change, empty the bootstrap allowlist and remove the token path
   from `release.yml`.

Trusted Publishing cannot be configured before a crate's first release. The
one-time token is therefore an explicit, bounded exception, not a permanent
fallback. See the official [crates.io Trusted Publishing announcement](https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/)
and [Cargo publishing rules](https://doc.rust-lang.org/cargo/reference/publishing.html).

### v13 package registration

`rullst-privacy`, `rullst-supervision`, `rullst-media` and `rullst-labs` were
first published in 13.0.0-alpha.1 on October 11, 2026, from release run
[38151068563](https://github.com/Rullst/Rullst/actions/runs/38151068563).
The [crates.io documentation](https://crates.io/docs/trusted-publishing) still
requires an API token for a first publication.

The first attempt (run 38121092958) refused before uploading anything, because
the publish job had no first-publication path. Registering the names outside
the workflow was not possible: `rullst-media` depends on `rullst-core` at the
same prerelease version. [PR #452](https://github.com/Rullst/Rullst/pull/452)
restored the fallback above for that release:
- The maintainer created a short-lived `publish-new` token and stored it only
  as the `crates-io` environment secret.
- All twenty archives matched the release checksums on crates.io.

A reviewed change after that release reverted the fallback and emptied the
allowlist. The maintainer then completed step 6 for the four crates:
- set the Trusted Publisher to `Rullst/Rullst`, `release.yml`, environment
  `crates-io`;
- enable trusted-publishing-only updates;
- revoke the token and delete the environment secret.

**Lesson:** check for unregistered names before the admission. The release
verification job accepted reviewed bootstrap names, so the refusal surfaced only
in the publish job, two hours into the run. Since then:
- `.github/release-preflight.py` refuses, before the admission is dispatched,
  every unregistered name that is not in the bootstrap allowlist, and the
  verification job refuses it before packaging;
- for an allowlisted name, both report the first publication and its
  credential instead, so a missing token is known before the tag.

### First publication of `cargo-rullst-extras` (13.0.0-alpha.4)

`cargo-rullst-extras` is a new, reviewed package name in the release order,
after `cargo-rullst`, and the only name in the bootstrap allowlist. It depends
on `cargo-rullst` at the same prerelease version, so, like `rullst-media` in
13.0.0-alpha.1, it cannot be registered outside the release that publishes that
version. The other twenty names are registered and keep Trusted Publishing.

Before the tag (steps 3 and 4 above):
1. Run `python3 .github/release-preflight.py v13.0.0-alpha.4` on the release
   head. It must report `cargo-rullst-extras: first publication; needs
   CRATES_IO_BOOTSTRAP_TOKEN in the crates-io environment` and refuse nothing
   else.
2. On crates.io, create an API token with only the `publish-new` endpoint
   scope and an expiry a few days after the planned tag. Restrict its crate
   scope to `cargo-rullst-extras` if the form accepts a name that is not
   registered yet; otherwise leave the crate scope empty and rely on
   `publish-new` and the short expiry.
3. Store it only as the `CRATES_IO_BOOTSTRAP_TOKEN` secret of the protected
   `crates-io` environment (Settings → Environments → `crates-io`), never as a
   repository or organization secret.
4. The preflight and the verification job cannot read an environment secret.
   After the environment approval, the publish job checks that the secret is
   set and stops before uploading anything when it is missing: add it and
   re-run the failed job.

After publication (step 6 above):
1. Check that `cargo-rullst-extras` 13.0.0-alpha.4 is indexed with the release
   checksum and that `venelouis` owns it.
2. Configure its Trusted Publisher (`Rullst/Rullst`, `release.yml`, environment
   `crates-io`) and enable trusted-publishing-only updates.
3. Revoke the token on crates.io and delete the `CRATES_IO_BOOTSTRAP_TOKEN`
   environment secret.
4. In a reviewed change, empty the bootstrap allowlist and remove the token
   path from `release.yml` (the publish job's credential check and per-package
   token selection), as after 13.0.0-alpha.1. The next release's ownership
   check then verifies that `venelouis` owns all twenty-one names.

## Determine the exact state

For each package in `.github/release-order.json`:

1. query `https://crates.io/api/v1/crates/<crate>/<version>`;
2. classify it as not indexed, indexed with the expected checksum, or indexed
   with an unexpected checksum;
3. compare the registry checksum to the retained verified archive;
4. verify that every already-published internal dependency precedes its
   dependent in the release order.

Do not infer success only from the exit status of `cargo publish`. Registry
indexing is asynchronous, so poll with a bounded timeout. The release workflow
performs this check after each package.

## Recovery decisions

### Nothing was accepted

Correct the failure on a new commit. If the tag or package content must change,
use a new prerelease version such as `12.0.0-rc.2`. Never move a published or
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

- every version in the tagged release inventory is indexed with its retained
  checksum (16 packages for v12; 21 in the current v13 candidate);
- a clean consumer resolves only registry packages and compiles;
- documentation/index pages are reachable;
- the incident timeline and any yanks/replacements are recorded;
- temporary credentials are revoked and Trusted Publishing is restored;
- the next release includes a regression for the initiating failure.

This process is package recovery, not application rollback. Database migrations,
traffic rollback, secrets and deployed binaries require a separate
application-specific operational plan.

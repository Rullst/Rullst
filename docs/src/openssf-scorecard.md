# OpenSSF Scorecard: evidence and improvement plan

The [public report](https://api.scorecard.dev/projects/github.com/Rullst/Rullst)
is separate from Rullst's [per-commit quality scorecard](quality-scorecard.md).
On 27 September 2026 at 09:52 UTC, OpenSSF Scorecard v5.5.0 reported **8.2/10**
for `6c926f95b5e6ba700ac7b094f30144c96d2b5fb7`. The README badge follows the
live report; this dated snapshot is not a promised future score or certification.

## Remaining checks

| Check | Snapshot | Evidence and next action |
| --- | --- | --- |
| Signed-Releases | 0/10 | GitHub stores genuine release attestations, but the release assets do not include portable signature bundles. The release workflow change for v12.1.2 and v13 preserves the original Sigstore bundle and verifies it before attaching it to future releases. Acceptance in a real tag workflow remains pending. |
| Code-Review | 0/10 | The scanner found no approved changesets among 17 sampled changesets. Arrange independent human review, especially for authentication, tenant boundaries and release infrastructure. AI analysis is additional evidence, not a second human reviewer. |
| Branch-Protection | Unavailable (-1) | The workflow token cannot read classic branch-protection settings. Protected `main` and `v12` do exist. A public ruleset now mirrors both branches’ existing requirements without adding a token or removing classic protections. Anonymous REST checks confirmed visibility and all 46 application-bound checks. The follow-up scanner recognized the rules and assigned 4/10; required human approvals remain absent. |
| SAST | 8/10 | The scanner detected CodeQL but credited only 12 of 30 commits. Both language analyses and GitHub's CodeQL findings check are required for merge. A historical first-page detection limitation is documented in the [delivery record](v13-delivery-plan.md); a fresh review of this snapshot found successful CodeQL suites on the heads of all 17 merged PRs associated with its 30 commits. All suites appeared within the first 30 results in the maintainer’s query, so the earlier pagination explanation does not establish the cause of this discrepancy. Scanner-token visibility/caching remains unconfirmed. |
| CII-Best-Practices | 5/10 | The project has the Passing badge. Silver is deferred while operational continuity is unresolved; it is not a v12.1.2 release objective. The [governance policy](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md) records current roles and the unresolved continuity requirement. A Silver badge is not claimed. |
| Binary-Artifacts | 9/10 | The finding identifies the [trusted Rust/Wasm compatibility fixture](https://github.com/Rullst/Rullst/blob/main/rullst-labs-runner/tests/fixtures/README.md). Required Labs CI reproduces it byte-for-byte with pinned Rust. Keep that coverage; replacing its storage requires a separate reproducibility design. |

The other eleven checks scored 10 in this snapshot. No check is disabled and no
badge score is hardcoded as part of this work. An unavailable check is missing
visibility, not a passing check. Making it observable can lower the aggregate
score while improving the accuracy of the report.

The [follow-up run at 19:07 UTC](https://github.com/Rullst/Rullst/actions/runs/36343168687)
reported **7.9/10** for `a8a78aa2ac79654e366930ae581cc2829922d9f4` after public
ruleset activation. Branch-Protection now contributes 4/10 instead of being
excluded as unavailable; no existing protection was removed. SAST remained 8/10
(13 of 30 commits detected). Signed-Releases remained 0 because the portable
provenance export has not yet been published. These observations do not establish
a future release score, and badge improvement is not a publication prerequisite.

## Portable release attestations

The release workflow already signs the verified crate archives, native CLI
artifacts and release evidence in a job that neither checks out source nor
executes downloaded artifacts. The added export copies that action's original
bundle to `rullst-release.sigstore.json`; it does not invent, re-sign or alter
the attestation. Its unchanged signed DSSE envelope is also exported as one
JSON line in `rullst-release.intoto.jsonl`, for in-toto consumers. Both jobs compare
that export against the original bundle and reject a mismatch. The signing job
verifies the exported bundle before registry publication.
An immutable workflow artifact then transports it to the GitHub release job,
which independently verifies every attested file after downloading it
against the bundle, the expected repository, exact release workflow/tag identity,
source and signer commits, GitHub's OIDC issuer, and hosted-runner requirement.
Any verification failure stops GitHub release creation.

This makes the signature, certificate and transparency evidence available beside
the artifacts, including for consumers retaining local release evidence. GitHub's
attestation service remains available; the CLI's existing verification policy is
unchanged. [GitHub CLI](https://cli.github.com/manual/gh_attestation_verify)
supports verification with a downloaded bundle:

```sh
# Supply an independently trusted tag and full commit, not values read only
# from an unverified download. ARTIFACT is a downloaded crate or CLI file.
gh attestation verify "$ARTIFACT" \
  --bundle rullst-release.sigstore.json \
  --repo Rullst/Rullst \
  --cert-identity "https://github.com/Rullst/Rullst/.github/workflows/release.yml@refs/tags/$RELEASE_TAG" \
  --cert-oidc-issuer https://token.actions.githubusercontent.com \
  --source-ref "refs/tags/$RELEASE_TAG" \
  --source-digest "$RELEASE_COMMIT" \
  --signer-digest "$RELEASE_COMMIT" \
  --deny-self-hosted-runners
```

A local bundle does not by itself supply the verifier's trusted roots. Follow the
GitHub CLI instructions when preparing completely offline verification.

Scorecard v5.5.0 [recognizes `.sigstore.json` release assets](https://github.com/ossf/scorecard/blob/v5.5.0/probes/releasesAreSigned/impl.go).
It also [recognizes `.intoto.jsonl` provenance](https://github.com/ossf/scorecard/blob/v5.5.0/probes/releasesHaveProvenance/impl.go).
Those filename heuristics are weaker than cryptographic verification, which is why
the release job verifies the actual bundle and artifacts. The score samples up
to five releases, so one future release does not establish a perfect history.
Historical releases and their attestations are not rewritten by this change.

## Sequencing and acceptance

These release improvements are included in v12.1.2 preparation. The prior source
`273db60cae95c397401bdd5a2d28ca88de96e7fc` passed 28/28 controls, but that is
historical evidence for that SHA. Release/provenance and documentation changes
require corresponding admission of the new source. Use the existing fuzz
equivalence policy; do not assume that unchanged Rust files imply reusable
evidence, alter campaign duration, or weaken a gate to save a run.

Before merging, require workflow lint, documentation checks and protected CI.
Rehearse the verifier with a genuine published artifact/bundle and confirm
rejection of changed bytes, an incorrect workflow identity and an incorrect
source commit. Final end-to-end acceptance requires a separately authorized tag
release, an attached bundle and successful consumer verification. Only a later
public Scorecard report can establish a score increase.

Independent human review is unavailable under the present sole-maintainer model.
Badge declarations and actual continuity arrangements remain maintainer
decisions. The [official check definitions](https://github.com/ossf/scorecard/blob/main/docs/checks.md)
and [restricted authentication guidance](https://github.com/ossf/scorecard-action/blob/main/docs/authentication/fine-grained-auth-token.md)
describe their criteria. Do not fabricate approvals or weaken protections to
improve a numerical score.

A Silver application is deferred. Retain the current Passing badge and revisit
that programme only when an actual continuity arrangement is available.

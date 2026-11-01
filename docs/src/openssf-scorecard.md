# OpenSSF Scorecard: evidence and improvement plan

The [public report](https://api.scorecard.dev/projects/github.com/Rullst/Rullst)
is separate from Rullst's [per-commit quality scorecard](quality-scorecard.md).
On 29 October 2026 at 15:37 UTC, OpenSSF Scorecard v5.6.0 reported **8.5/10**
for `27440eb88706fe739bf7cc5a72908e61c45d6f9a`. The README badge follows the
live report; this dated snapshot is not a promised future score or certification.

## Remaining checks

| Check | Snapshot | Evidence and next action |
| --- | --- | --- |
| Code-Review | 0/10 | The scanner found no approved changesets among 7 sampled changesets. Arrange independent human review, especially for authentication, tenant boundaries and release infrastructure. AI analysis is additional evidence, not a second human reviewer. |
| Branch-Protection | 3/10 | On `main` the scanner found required pull requests and status checks, administrator enforcement, stale-review dismissal and blocked force pushes and deletion. It deducts for no required approvers, no code-owner review, no last-push approval and no up-to-date requirement. Since 31 October 2026 `main` requires 20 checks without an up-to-date base (19 from 29 September 2026, before the ORM differential databases check was added) ([development-branch ruleset](https://github.com/Rullst/Rullst/rules/24175738)); `v12` keeps 46 checks and an up-to-date base ([maintained-branch ruleset](https://github.com/Rullst/Rullst/rules/24080901)). Required human approvals remain absent while there is one active maintainer (see the [governance policy](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md#change-and-release-controls)). |
| SAST | 7/10 | CodeQL is configured, but the scanner found none of the last 30 commits checked. Pull requests to `main` skip CodeQL, and its checks are not required there; it runs after each push to `main`, weekly and on pull requests to `v12`. |
| Vulnerabilities | 9/10 | One OSV record, [GHSA-qwgh-2vcv-g2f7](https://osv.dev/GHSA-qwgh-2vcv-g2f7), covers `block-buffer` before 0.12.1. `Cargo.lock` still resolves `block-buffer` 0.10.4 through `digest` 0.10 (used by `sha1`, `sha2` 0.10 and `curve25519-dalek` 4). The record has no RustSec identifier, so `cargo deny` and `cargo audit` do not report it. Next action: assess whether the 0.10 line is affected and move those dependents off it when their releases allow. |
| CII-Best-Practices | 7/10 | The Silver badge, granted on 10 October 2026, is detected; Gold is the next level. |

The other thirteen checks scored 10 in this snapshot, including Signed-Releases
(signed bundles and provenance on the last five releases, v12.2.0 to
v13.0.0-alpha.3) and Binary-Artifacts (the Labs runner's Wasm fixture left the
repository with the runner candidate). No check is disabled and no badge score
is hardcoded. An unavailable check is missing visibility, not a passing check.
Making it observable can lower the aggregate score while improving the accuracy
of the report.

Earlier snapshots, for comparison: on 27 September 2026 Scorecard v5.5.0
reported 8.2/10 for `6c926f95b5e6ba700ac7b094f30144c96d2b5fb7`, with
Branch-Protection unavailable to the workflow token. The public rulesets made it
observable, and the [follow-up run](https://github.com/Rullst/Rullst/actions/runs/36343168687)
the same day reported 7.9/10 for `a8a78aa2ac79654e366930ae581cc2829922d9f4`
(Branch-Protection 4/10, Signed-Releases 0/10 before the portable bundle was
published). These observations do not establish a future release score, and
badge improvement is not a publication prerequisite.

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

Scorecard [recognizes `.sigstore.json` release assets](https://github.com/ossf/scorecard/blob/v5.6.0/probes/releasesAreSigned/impl.go).
It also [recognizes `.intoto.jsonl` provenance](https://github.com/ossf/scorecard/blob/v5.6.0/probes/releasesHaveProvenance/impl.go).
Those filename heuristics are weaker than cryptographic verification, which is why
the release job verifies the actual bundle and artifacts. The score samples up
to five releases. Releases before v12.1.2 and their attestations were not
rewritten.

## Acceptance

The export first shipped with v12.1.2, and every release since (v12.2.0,
v12.3.0 and 13.0.0-alpha.1 to alpha.3) attaches both files. On 29 October 2026
the command above, with `RELEASE_TAG=v13.0.0-alpha.3` and the tag's commit
`be33683252d29244ea14311790ac66decaf9d299`, verified
`rullst-macros-13.0.0-alpha.3.crate` against that release's bundle and rejected
a copy with one byte appended. Signed-Releases scores 10 in the current
snapshot.

A change to the release or provenance steps still needs admission of the new
source under the [release guide](https://github.com/Rullst/Rullst/blob/main/RELEASE_GUIDE.md).
Use the existing fuzz equivalence policy; do not assume that unchanged Rust
files imply reusable evidence, alter campaign duration, or weaken a gate to save
a run. Only a later public Scorecard report can establish a score change.

Independent human review is still rare: there is one active maintainer, and the
successor maintainer's approvals count only for changes they actually review.
Badge declarations and actual continuity arrangements remain maintainer
decisions. The [official check definitions](https://github.com/ossf/scorecard/blob/main/docs/checks.md)
and [restricted authentication guidance](https://github.com/ossf/scorecard-action/blob/main/docs/authentication/fine-grained-auth-token.md)
describe their criteria. Do not fabricate approvals or weaken protections to
improve a numerical score.

bestpractices.dev granted the Silver badge on 10 October 2026, with every
criterion answered. The [Silver evidence guide](openssf-best-practices.md)
records the answer, justification and evidence for each criterion. Keep it
current when an answer changes.

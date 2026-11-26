# How Rullst is verified, and what that does not prove

> 🇧🇷 [Leia esta página em português](pt-BR/how-rullst-is-verified.md).

This page is for a developer who wants to know how far to trust Rullst. It
lists the checks that run before a release, the evidence each one leaves, and
what none of them shows. Every statement links to its evidence. Where a check
has a limit, the limit is stated next to it.

Nothing here is an audit, a certification or an external review. No such
review has happened. Passing these checks means that the code does what its
tests and its specification say. It does not mean the design is right, that
the specification is complete, or that an application you build is secure.

The facts below were checked against the repository, the release and the
workflow runs on 26 November 2026, for
[13.0.0-rc.1](https://github.com/Rullst/Rullst/releases/tag/v13.0.0-rc.1).
Later releases change the numbers; their changelogs say so.

## The layers

### 1. The maintainer's applications

Rullst is used in the maintainer's own applications. Academy, capoeira and
vene.win run on real domains on one Oracle Ampere A1 Arm machine behind a
shared Caddy with Let's Encrypt certificates
([Scope and Limits](scope-and-limits.md#external-accounts-and-providers)).
The maintainer also runs a private application on Rullst. Google and GitHub
sign-in and the Bunny Stream adapter of `rullst-media` are used by the
maintainer's production applications
([maturity tiers](maturity.md#what-the-documentation-tells-you)).

What this shows: upgrading the applications to 13.0.0-alpha.6 exposed defects
that the tests had missed, and the defects they reported were fixed in rc.1
([#781](https://github.com/Rullst/Rullst/pull/781),
[changelog](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md#1300-rc1---2026-11-24),
[v13 delivery plan](v13-delivery-plan.md)).

What it does not show: the applications and the framework have one author, so
this is not independent use. No automated test runs against these
deployments, and what they do not use is not covered.

### 2. Tests and required checks

Every pull request to `main` runs tests, strict Clippy, feature-boundary
checks, the zero-panic policy, the unsafe-code policy, the MSRV check,
dependency policy and secret scanning. `main` requires 20 named checks before
a merge ([`GOVERNANCE.md`](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md#change-and-release-controls)).
Coverage runs after a merge to `main` (and weekly), not before it, and it is
release-required. It is measured with LLVM, and the workflow rejects a summary
below 90% for the repository or for the framework libraries
([Coverage](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#coverage),
[`coverage.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/coverage.yml)).
The zero-panic and unsafe-code workflows are described in
[Zero-panics and unsafe Rust](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#zero-panics-and-unsafe-rust):
the unsafe policy is an enforced boundary, not a claim that dependencies
contain no unsafe Rust.

What it does not show: a test passes when the code matches what the test
expects. A wrong expectation passes too.

### 3. Mutation testing

[`mutants-pr.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/mutants-pr.yml)
changes the Rust lines a pull request changes, one small change (a mutant) at
a time, and fails when no test notices (a missed mutant). It skips the globs in
`.github/mutants-pr-exclude.txt` and the `cargo-rullst` files not listed in
`.github/mutants-pr-cli.txt`. Timeouts and unviable mutants do not fail it.
Branch protection does not require it, and it is not part of release
admission ([workflow inventory](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#workflow-inventory-36-definitions),
[pull-request mutation gate](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#pull-request-mutation-gate)).
The manual
[`mutants.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/mutants.yml)
runs full and targeted campaigns, and a mutant that survives either gets a
test or is listed, with its reason, as an equivalent mutant. The
[mutation testing record](mutation-testing.md) lists the campaigns up to
13.0.0-alpha.6 and their accepted equivalents. The rc.1 re-measurement is
recorded in [PR #872](https://github.com/Rullst/Rullst/pull/872), not in that
file.

For 13.0.0-rc.1, targeted campaigns re-measured ten `cargo-rullst` files on
the maintainer's fork. They had 732 mutants: 579 caught, 6 timeouts and 98
missed, the rest unviable. Of the 94 distinct missed mutants, 80 received a
test and 14 were classified as equivalent or unobservable
([PR #872](https://github.com/Rullst/Rullst/pull/872)). A re-measurement of the
ten files on that branch left 21 missed mutants and 6 timeouts, all within the
categories the pull request classified as equivalent or unobservable
([comment on PR #872](https://github.com/Rullst/Rullst/pull/872#issuecomment-7195513104)).

What this does not cover:

- Code merged after the campaigns' source commit, `a0ee8c3f`, was in no
  campaign. The pull-request gate mutated the lines it touched in the library
  crates and in the listed `cargo-rullst` files; the other `cargo-rullst`
  files were tested but not mutated.
- Gaps that the rc.1 Known limitations list: 30 `cargo-rullst` mutants from
  its first campaign, the CLI's frontend toolkit and parts of `footprint`
  outside the gate, one unresolved `rullst-orm` mutant and 17 timeouts, 29
  `rullst-capital` mutants reachable only through a live Stripe response, and
  three `rullst-ai` mutants reachable only through a live HTTPS origin
  ([changelog, Known limitations](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md#1300-rc1---2026-11-24)).
- "Equivalent" is the maintainer's judgment, recorded with a reason, not a
  proof. A surviving mutant that only a live provider response reaches is left
  to provider testing
  ([MAINTAINING.md](https://github.com/Rullst/Rullst/blob/main/MAINTAINING.md#6-mutation-campaigns)).

### 4. Fuzzing

`fuzzing.yml` runs libFuzzer on 42 declared targets in eleven fuzz workspaces
([Fuzzing and OSS-Fuzz](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#fuzzing-and-oss-fuzz)).
The workflow is started by hand for a release; it is not a pull-request
check. A release needs a successful run on the release commit. For 13.0.0-rc.1 that
is [run 45693088611](https://github.com/Rullst/Rullst/actions/runs/45693088611)
on commit `9ea50d98`: 42 targets selected, none reused from an earlier run
(the `fuzz-evidence-9ea50d98…` artifact of that run records it), each with a
campaign budget of 19,800 seconds.

What it does not show: fuzzing is bounded random testing. It is not proof that
an input is safe, or that another run would find nothing
([fuzz evidence reuse](fuzz-evidence.md#publication-and-receipts)). The
`oss-fuzz/` directory is a local draft; upstream OSS-Fuzz has not accepted the
project.

### 5. Miri, Kani, sanitizers and property tests

These run on a schedule (Kani and Miri weekly, the sanitizers daily) or are
started by hand for a release; none is a pull-request check
([workflow inventory](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#workflow-inventory-36-definitions)).

- **Kani** proves bounded properties of pure decisions with twenty harnesses
  in ten packages. **Miri** runs 15 named pure-Rust scopes. Neither covers
  native crypto, SQLite, network paths or whole concurrent middleware.
- **ThreadSanitizer and AddressSanitizer** run across twelve packages. There
  is no MemorySanitizer job.
- **Property tests** (`proptest.yml`) run the all-feature suite in release mode
  with 10,000 cases for the ORM and Connect contracts.

All four succeeded on commit `9ea50d98`
([Formal, dynamic and stress analysis](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#formal-dynamic-and-stress-analysis);
workflows
[`kani.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/kani.yml),
[`miri.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/miri.yml),
[`sanitizers.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/sanitizers.yml),
[`proptest.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/proptest.yml)).

What they do not show: they are bounded evidence for the harnesses and scopes
that run.

### 6. Static and dynamic security scans

- **CodeQL** (`codeql.yml`) analyses the Rust and the JavaScript/TypeScript
  code after each push to `main`, weekly, and on pull requests to the `v12` and
  `v13` branches. A pull request to `main` does not run it, and it is not a
  check that `main` requires
  ([`codeql.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/codeql.yml)).
- **OWASP ZAP** (`dast-zap.yml`) scans a freshly generated REST API starter and
  SaaS starter (public pages, then signed-in pages) and the LMS example. It
  is started by hand for a release, not on pull requests. The
  targets are representative, not every starter, role or deployment; Nexus
  administrators are not scanned
  ([`dast-zap.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/dast-zap.yml),
  [Formal, dynamic and stress analysis](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#formal-dynamic-and-stress-analysis)).
- **SonarCloud** analyses pull requests. It is not a required check: it is not
  among the checks `main` requires
  ([ruleset](https://github.com/Rullst/Rullst/rules/24175738)).

CodeQL and ZAP are among the workflows that must pass for a release
([`release-required-workflows.json`](https://github.com/Rullst/Rullst/blob/main/.github/release-required-workflows.json)).
SonarCloud is not.

What they do not show: scanners find known patterns. They do not find a flaw
in a design they cannot see.

### 7. Dependency audit and release admission

- `cargo deny` and `cargo audit` run in
  [`supply-chain.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/supply-chain.yml).
  Two advisories have time-limited exceptions until 2027-01-28, listed with
  their reasons in [Security Advisory Exceptions](security-advisory-exceptions.md).
- OSV-Scanner runs through the OpenSSF Scorecard workflow
  ([`scorecards.yml`](https://github.com/Rullst/Rullst/blob/main/.github/workflows/scorecards.yml));
  its ignores are listed in the same page. The score is supply-chain evidence,
  not a security certification
  ([OpenSSF Scorecard](openssf-scorecard.md)).
- **Release admission** (`release.yml`, tag-only) refuses to publish unless 23
  workflows named in
  [`release-required-workflows.json`](https://github.com/Rullst/Rullst/blob/main/.github/release-required-workflows.json)
  succeeded on the exact commit. For 13.0.0-rc.1 they did, on commit
  `9ea50d98`, and
  [release run 45837583222](https://github.com/Rullst/Rullst/actions/runs/45837583222)
  published all twenty packages
  ([v13 delivery plan](v13-delivery-plan.md)). The release carries the SBOM,
  the `cargo audit` output and a GitHub build-provenance attestation. This is
  not an SLSA level, a signing claim or a regulatory compliance claim
  ([Supply chain and release provenance](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md#supply-chain-and-release-provenance)).

The admission of the first rc.1 candidate found a stale threat-model path and
a RustSec advisory (`url`); the second candidate met another (`cookie`). Both
were fixed before the tag ([#862](https://github.com/Rullst/Rullst/pull/862),
[#867](https://github.com/Rullst/Rullst/pull/867)).

### 8. Skeptic reviews of the public claims

Before a release, an AI assistant without the release work's context reads the
public documentation and the code, tests and CI, and reports what the
documentation claims but does not support. Each finding is fixed or stated in
the known limitations
([MAINTAINING.md](https://github.com/Rullst/Rullst/blob/main/MAINTAINING.md#7-releasing-a-prerelease)).
For rc.1 the record is in four merged pull requests:
[#784](https://github.com/Rullst/Rullst/pull/784) aligned the public claims
with the code, tests and CI,
[#855](https://github.com/Rullst/Rullst/pull/855) corrected the rc.1 claims
the pre-release review found,
[#857](https://github.com/Rullst/Rullst/pull/857) narrowed the site's claims
and [#856](https://github.com/Rullst/Rullst/pull/856) fixed a CLI defect
(`--from` without a project version).
[`GOVERNANCE.md`](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md#change-and-release-controls)
says plainly that an assistant is not an independent human reviewer.

## What this does not prove

- **Design and claims.** The layers check that the code does what its tests and
  its specification say. They do not show that a design decision, the
  specification or a public claim is right.
- **No external security review.** No third party has reviewed Rullst's
  security. The independent security review is open work on the
  [roadmap](https://github.com/Rullst/Rullst/blob/main/ROADMAP.md#verification-roadmap),
  and the [external audit kit](external-audit-kit.md) is the material for a
  reviewer. Rullst's own tools do not certify either: the
  [security evidence](https://github.com/Rullst/Rullst/blob/main/SECURITY.md)
  commands print what they checked and mark the rest as not evaluated.
- **No live provider account in automated tests.** CI exercises payment,
  video, OAuth, mail and AI providers against local mocks, fixtures and test
  servers
  ([Scope and Limits](scope-and-limits.md#external-accounts-and-providers),
  [maturity tiers](maturity.md#what-the-documentation-tells-you)). Several
  providers have never run against a real account at all: Microsoft, Apple,
  Auth0, AWS Cognito, Facebook, X, Discord, LinkedIn and generic OpenID Connect
  sign-in; OpenAI, Anthropic Claude, DeepSeek and Ollama; AWS SES and
  SendPulse; and the S3/R2 adapter of `rullst-media`. Turso, Meilisearch,
  SurrealDB, Redis, MySQL and pgvector ran only against local containers. See
  the [changelog's rc.1 Known limitations](https://github.com/Rullst/Rullst/blob/main/CHANGELOG.md#1300-rc1---2026-11-24)
  and the tracking issue [#801](https://github.com/Rullst/Rullst/issues/801).
- **One maintainer.** One person owns the code, the tests, the
  specification and the checks, and merges. Required approvals on `main` are
  zero while there is one active maintainer
  ([`GOVERNANCE.md`](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md#change-and-release-controls)).
  Assistants prepare changes within the maintainer's authority, and
  [`GOVERNANCE.md`](https://github.com/Rullst/Rullst/blob/main/GOVERNANCE.md#change-and-release-controls)
  says that they are not an independent reviewer. A check can only catch what
  the maintainer thought to check.
- **Your application.** None of this covers the code you write, your
  deployment, your data or your configuration.

## Case study: verification is not validation

Verification asks whether the work meets its checks. Validation asks whether
the checks measure the right thing. The Rullst
[Benchmarks](https://github.com/Rullst/Benchmarks) repository shows the
difference twice.

**Version 1 measured the wrong thing.** The first measurement, made by hand on
a laptop on 22 June 2026, listed a "Passed" status for entries such as FastAPI
at 2.49 requests per second on `/db-single`. It had no conformance check, no
recorded configuration and no reproducible output, and the repository now
calls several entries "evidently broken". It is kept only as a record and is
marked not comparable
([history](https://github.com/Rullst/Benchmarks/blob/main/history/2026-06-22-laptop.md)).
Its successor, version 2, was also found to measure less than it seemed: its
`/db-single` tier was client-bound in most rounds, so the load generator, not the
server, limited the result
([Benchmarks History](https://github.com/Rullst/Benchmarks#history)).

**The `PUT /world/{id}` flaw favoured Rullst.** Version 3 passed its own
conformance checks, and its first official run (Rullst 13.0.0-alpha.6)
published results. An assistant-run skeptic review (the maintainer's AI assistant, without the
release's context; not an external or human review) on 20 November 2026 found that the
`PUT` endpoint measured no write for the implementations whose ORM writes only
changed columns, Rullst among them: the load repeated pairs that the warm-up
had already stored. Rullst's `PUT` ratio and Application score were higher
than they would be with a write. The affected results are marked ‡ until a
later run replaces them, every `PUT` now carries a new value, and the
publication gate refuses a run that did not write
([Benchmarks History](https://github.com/Rullst/Benchmarks#history),
[footnote ‡](https://github.com/Rullst/Benchmarks#readme),
[v3 specification](https://github.com/Rullst/Benchmarks/blob/main/docs/v3-spec.md)).
The same README says that the maintainer wrote every implementation and
designed the method.

The lesson for this page: a green check says the check passed. Whether the
check was the right one is a separate question, and an assistant-run review, not
the benchmark's own checks, found the `PUT` flaw.

## Check it yourself

- **Workflow runs.** The
  [Actions tab](https://github.com/Rullst/Rullst/actions) holds every run
  linked above. [`WORKFLOWS.md`](https://github.com/Rullst/Rullst/blob/main/WORKFLOWS.md)
  (the book's [workflows](workflows.md) page) says what each workflow does and
  does not establish.
- **Fuzz evidence.** The fuzzing run of a release uploads a
  `fuzz-evidence-<commit>` artifact that lists the candidate, the policy and
  input digests, and each target either selected or reused with its original
  run ([fuzz evidence reuse](fuzz-evidence.md)). The admission check also retains a
  `release-fuzz-evidence.json` for its decision; for 13.0.0-rc.1 the record to
  read is the artifact of run 45693088611.
- **Admission.** [`release-required-workflows.json`](https://github.com/Rullst/Rullst/blob/main/.github/release-required-workflows.json)
  names the workflows, and `python3 .github/check-release-admission.py` reports
  what is missing for a commit ([MAINTAINING.md](https://github.com/Rullst/Rullst/blob/main/MAINTAINING.md#7-releasing-a-prerelease)).
- **Release files.** The [13.0.0-rc.1 release](https://github.com/Rullst/Rullst/releases/tag/v13.0.0-rc.1)
  attaches the SBOM, `cargo-audit.json`, checksums and the provenance
  attestation.
- **Benchmarks.** The [Benchmarks repository](https://github.com/Rullst/Benchmarks)
  publishes its method, raw output and limits.
- **Mutation record.** [Mutation testing record](mutation-testing.md) lists
  the campaigns up to 13.0.0-alpha.6 and their accepted equivalent mutants;
  [PR #872](https://github.com/Rullst/Rullst/pull/872) holds the rc.1
  re-measurement.

If a statement on this page is not supported by its link, or if you find a
flaw, [open an issue](https://github.com/Rullst/Rullst/issues/new/choose).
Report security vulnerabilities privately, as
[`SECURITY.md`](https://github.com/Rullst/Rullst/blob/main/SECURITY.md)
describes.

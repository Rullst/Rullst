# Verus verification pilot for v13

**Status: locally verified v13 candidate, 20 September 2026.** One production
method predicate and three deliberate failing controls have passed the pilot
harness. A manual workflow is prepared; hosted acceptance and any promotion
remain pending. There are no new runtime dependencies. It adds no v12.1 gate and
preserves the existing Kani, Miri, fuzzing, integration and provider acceptance
requirements.

The first property uses the existing `AgePolicy::permits` implementation. Expand to
Auth and Capital only after measuring compatibility, proof maintenance and CI
cost. The first privacy/age journey retains its product priority.

## Implemented candidate and measured boundary

The source-linked projection verifies exactly this engineering policy table:

| Risk preset | Self-declaration | Facial estimation | Verified attribute |
| --- | --- | --- | --- |
| Low | Allowed method | Allowed method | Allowed method |
| Elevated | Denied method | Allowed method | Allowed method |
| Restricted | Denied method | Denied method | Allowed method |

An allowed **method** does not authorize an application action or establish a
person's age. The predicate receives an `AgePolicy` and an `AgeMethod`; it does
not inspect evidence, signatures, challenges, replay state, clocks or guardians.
Those boundaries retain their runtime/integration tests and separate contracts.

The specification lives in `rullst-privacy/verification/age-policy.spec.rs`.
An ordinary CLI integration test parses actual production Rust with `syn`, checks
the package/root/module/export path and the exact reviewed enum/type/signature
domain, and extracts the executable body unchanged. It rejects new calls,
macros, unsafe code or conditional attributes until their proof closure is
reviewed. Type/body/source/specification fingerprints accompany the projection.
The only additions are Verus annotations, `Structural` support for the existing
fieldless derived-equality enums, a closed specification accessor and an erased
reveal step. Production source and normal package dependencies remain unchanged.
There is no handwritten copy of the decision algorithm and no proof precondition
that excludes one of the nine risk/method combinations.

The trusted boundary includes the syntax extractor, Rust's derived structural
equality, Verus/its proof library, Rust compiler, Z3 and the executing host.
`--no-cheating` rejects local `assume`, admitted properties and external bodies;
the imported verifier standard library remains part of the trusted toolchain.
The runner requires the whole projection, exactly one verified production
function and its actual successful result. The three negative implementations
weaken restricted/elevated policies or deny low-risk methods. Each must fail
that function's postcondition; an absent proof, compiler error, zero-test pass
or changed tool/source cannot substitute for a functioning negative control.

The pinned Linux bundle is Verus `0.2026.09.13.671956e`, upstream commit
`671956ec527d3b7164779f767bdbfe769bedce6c`, Rust `1.98.1` and Z3 `4.16.0`.
The archive digest, flags and inventory are in `.github/verus-toolchain.json`.
Every run checks the archive and all extracted members before executing them.
Ordinary application builds still use the framework's own stable/MSRV policy
and do not install Verus.

Two initial local runs took 12.105 and 12.139 seconds including full bundle
validation. In the first, the production proof took 0.82 seconds and each
negative control about 0.53–0.54 seconds; peak prover RSS stayed below 292 MiB.
The compressed tool download is about 486 MB and expands to 1.63 GB. These are
measurements on one workstation, not hosted-CI or cold-download predictions.
Execution uses one verifier thread, a 60-second wall limit per case, 45-second
CPU limit, 2 GiB address-space limit and 2 MiB output-file limits. Local runtime
age tests and a Rust 1.96 age-feature check also passed.

## Running the isolated pilot

On Linux, download the exact URL from `.github/verus-toolchain.json` into a
disposable directory, install the pinned Rust toolchain and use fresh absolute
output paths. The installer verifies the archive before extraction:

```text
python3 .github/verus-pilot.py install --archive /absolute/release.zip --output /absolute/tools
RULLST_VERUS_OUTPUT=/absolute/projection cargo test -p cargo-rullst --test verus_policy_linkage --locked
python3 .github/verus-pilot.py verify --archive /absolute/release.zip --bundle /absolute/tools/verus-x86-linux --projection /absolute/projection --output /absolute/evidence
```

`evidence.json` records source SHA, dirty-worktree status, source/spec/type/body
linkage, exact tool identities/flags, result hashes, wall/CPU time and peak RSS.
Dirty local evidence is explicitly developmental. The prepared `verus.yml`
workflow uploads the projection and reports for its exact selected commit;
availability through GitHub's manual workflow registry and a successful hosted
run must be established before citing hosted evidence. It remains outside the
required release workflow inventory.

## Subsequent scope, not proven by this pilot

| Module | Proposed property | Explicit boundary |
| :--- | :--- | :--- |
| `rullst-privacy::age_assurance` | A restricted policy permits only a verified age attribute; expired, inconclusive or unavailable evidence cannot authorize an action requiring valid evidence. Threshold arithmetic stays within validated bounds. | Server time, authenticated subject/tenant binding, issuer trust, signature verification and atomic durable replay claims must have explicit contracts. Facial accuracy, document authenticity, guardianship and legal compliance are separate evidence. |
| `rullst-auth` | The selected authorization predicate rejects expired sessions and revoked session versions, given authoritative revocation state. | State freshness, persistence, races, cryptographic implementations and application routing are outside a pure predicate proof unless separately specified and verified. |
| `rullst-capital` | Selected integer money calculations respect currency scale, amount limits and the documented rounding rule; unrepresentable results return a typed error without overflow or silent truncation. | Provider prices, taxes, exchange rates, webhook delivery, distributed effects and live payment acceptance remain external or integration contracts. |

Begin with the existing age-policy method and threshold decisions, then the
evidence decision boundary. Identify the exact Auth and Capital functions before
expanding the inventory. Review preconditions against public input validation;
excluding a difficult valid input is not a fix for a failed proof.

## Code and package placement

Keep specifications and proof support adjacent to the owning module. Verify
the executable functions compiled into the library; any separate abstract model
needs a checked connection to those functions before it can support a production
claim. A passing copy of an algorithm is only model evidence.

No new public framework crate is planned solely for Verus. The adoption review
must choose and test how annotations, proof support and any required macros fit
the existing packages. Keep the verifier toolchain isolated and validate stable
Rust/MSRV, feature combinations and packaged consumer builds before accepting
dependency or source changes. Ordinary application builds must not require
installing the verifier.

Every proof records its property, executable entry points, valid input domain,
external contracts and trusted assumptions. Review `assume`, external bodies,
axioms and unverified dependencies explicitly. An unfinished proof or an
assumed target property cannot count as successful verification. Preserve
runtime validation at boundaries reached from unverified callers.

## CI adoption and promotion

The candidate dedicated `verus.yml` workflow uses `workflow_dispatch` and
a small named proof inventory. Its checks fail on a proof failure, timeout,
missing proof or tool error; the pilot remains outside release admission until
promotion is explicitly implemented.

- Pin the verifier, compatible Rust toolchain, proof libraries and solver;
  verify downloaded artifacts and retain their identities.
- Use bounded concurrency, per-proof time/resource limits and cancellation of
  superseded runs. Measure cold/warm duration, memory, disk and maintenance cost
  before expanding the matrix.
- Retain the exact source SHA, proof inventory, assumptions, dependency/tool
  versions, flags and results. Cached builds never substitute for checking the
  candidate's proofs.
- Exercise representative incorrect implementations to show that the intended
  properties fail verification. Retain ordinary behavioral and integration
  tests for the surrounding application boundary.
- Promote only after repeated reproducible runs, reviewed assumptions,
  production-code linkage, consumer compatibility and an agreed CI budget.
  Assign maintenance ownership and document the tool-upgrade procedure.
- After promotion, require verification for changes to the selected code,
  specifications, relevant callers/contracts, transitive dependencies, locks or
  verification tooling. Unknown impact must trigger the complete proof
  inventory. Required-check wiring must not leave skipped checks indefinitely
  pending or accept a missing run as proof.

## Delivery checkpoints

- [x] Select the first age-policy method predicate and review its threat model,
  contracts, assumptions and compatibility with a pinned Verus version.
- [x] Implement source-linked local proof and negative controls, stable runtime
  tests and an MSRV check without changing production dependencies.
- [x] Prepare the manual workflow and local resource/evidence collection.
- [ ] Obtain clean exact-commit hosted evidence and the combined package-consumer
  rehearsal; the local dirty-worktree samples are not release admission.
- [ ] Review results and decide whether to make the selected scope mandatory on
  relevant v13 changes.
- [ ] Evaluate the Auth and Capital predicates independently before expanding
  the scope or any release requirement.

A result supports only the named properties under their documented assumptions.
It does not establish that the whole framework is formally verified, that an
age estimate is accurate, or that a deployment complies with privacy law.

## References

- [Verus project and supported-language boundary](https://github.com/verus-lang/verus)
- [Specifications, assumptions and external bodies](https://verus-lang.github.io/verus/guide/requires_ensures.html)
- [Rullst privacy and age-assurance delivery plan](privacy-age-assurance-roadmap.md)
- [Current Rullst verification contract](workflows.md)

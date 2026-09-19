# Verus verification pilot for v13

**Status: planned v13 pilot, 18 September 2026.** No Verus proofs, dependencies
or workflow are implemented by this plan. It adds no v12.1 release gate and
preserves the existing Kani, Miri, fuzzing, integration and provider acceptance
requirements.

The pilot will verify a small set of explicit correctness properties in code
used by production modules. Start with the age-policy foundation; expand to
Auth and Capital only after measuring compatibility, proof maintenance and CI
cost. The first privacy/age journey retains its product priority.

## Initial scope

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

The proposed dedicated `verus.yml` workflow begins with `workflow_dispatch` and
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

- [ ] Select the first age-policy functions and review their threat model,
  contracts, assumptions and compatibility with a pinned Verus version.
- [ ] Implement proofs attached to the production functions, with negative
  controls and stable/MSRV/package-consumer checks.
- [ ] Add the manual workflow, exact-commit evidence and resource measurements.
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

# Rullst 12.1.2 maintenance candidate

Status: unpublished candidate prepared on 26 September 2026. The supported
stable line remains `v12`; `main` develops v13. Package versions, passing PRs
and this plan do not create a tag, registry release or security advisory.
The [release record](v12.md) establishes publication status separately.

## Scope and compatibility

The candidate builds on published 12.1.1 source
`d27db26c6089e06366ca01b67d741f3d037c076a`. One runtime correction is the Nexus
table renderer: escape every stored text value, including values beginning
with an HTML numeric-entity prefix. Ordinary Unicode boolean labels retain
their visible meaning without an escaping exception.

The HTML procedural macro also isolates its generated buffer identifier from
caller bindings. Previously, a caller variable named `s` could resolve to the
generated `String`, causing field-access/type errors or rendering the buffer's
contents when types happened to match. Regression tests cover text, dynamic
attributes, void/nested elements, fragments, iterator closures, nested macro
calls and single evaluation of mutable caller expressions. Facade tests retain
the real escaping and explicit `RawHtml` contract. This is a compile/render
correctness defect; no additional security impact has been established.

The demonstrated defect is stored HTML injection. An attacker must be able
to influence a displayed value and have it rendered in the administration
view. Actual JavaScript execution depends on the application and browser's
defenses; neither production exploitation nor a critical CVSS rating has
been established. Treat the correction as high-priority security maintenance.

Additional post-12.1.1 changes strengthen Auth, HTTP lifecycle and SQLite
messaging regression tests, including mutation-survivor cases and bounded
test orchestration. They do not add runtime features. All sixteen publication
packages and their internal requirements are prepared as 12.1.2; workspace
and ten fuzz locks must remain synchronized. External dependency versions,
public APIs and Rust **1.96.0** MSRV are preserved. No schema migration, real
provider integration or v13 capability is part of this patch.

## Evidence already obtained

- [Stable correction PR #275](https://github.com/Rullst/Rullst/pull/275) passed
  its hosted source checks and was integrated as
  `06ab959fc43ffab4bc764f62f5c15ebbffc603d3`.
- [Stable CI run 36213539488](https://github.com/Rullst/Rullst/actions/runs/36213539488)
  exercised the actual stored-value renderer and Chromium DOM assertions for
  72 text fixtures, numeric/boolean display and table structure. Portable and
  strict SQLite profiles are covered; the regression failed on the old renderer.
- [Development correction PR #276](https://github.com/Rullst/Rullst/pull/276)
  independently passed hosted checks and was integrated into `main` as
  `ab56500ab4275ac54861afdfdde0b41f11cf4e4e`.

Those checks concern the Nexus correction's source commits. They do not certify
the subsequent version/packaging candidate, installed CLI, final native
artifacts or release admission. Passing regressions do not establish the
absence of other flaws.

The later HTML macro correction requires a new candidate SHA and corresponding
validation. Final campaigns requested for `1dce103a` were cancelled after the
caller-binding defect was independently reproduced, to avoid spending long
campaigns on superseded source. Retain their results as history, not admission
of the corrected candidate.

## Required admission before a release decision

1. Review the complete diff from the published 12.1.1 tag. Keep original
   publication receipts, tags and checksums immutable. Verify all package and
   internal requirement versions, lockfiles and packaged README links.
2. Pass full workspace tests on the required platforms, strict all-feature
   Clippy, formatting, actual Rust 1.96.0 compilation, SemVer, coverage,
   dependency/security scans and every protected-branch requirement.
3. Validate archives for all sixteen packages, archive-only consumers,
   isolated installed CLI/blueprints and both CLI version entry points.
   Complete the required native artifact checks on the final source.
4. Run the existing source-bound admission verifier and reach **28/28** for
   the exact final `v12` SHA. Complete its required manual controls, including
   Kani, Miri, sanitizers and fuzz evidence. PR CI and an archive diagnostic
   cannot substitute for this release admission.
5. Use the existing fuzz equivalence verifier for all 40 targets. Reuse only
   eligible, unexpired, input-equivalent campaign evidence. Changed or
   unproven inputs require fresh campaigns. Any scope-policy refinement is a
   separate reviewed and tested change; do not silently exempt Nexus,
   documentation, workflow changes or version metadata. The existing Nexus
   fuzz harness targets identifier validation, so it does not replace the
   renderer/browser regression.
6. Retain a final report with SHA/tree, workflow URLs, archive and CLI results,
   limitations and outstanding decisions. Obtain the separate publication
   decision; this preparation does not authorize tags, crates.io publication
   or security-advisory publication.

Use hosted CI for large matrices and preserve the local AGENTS disk reserve.
Fixtures must remain synthetic and offline; do not use real provider accounts
or downstream production data. The final candidate has not yet completed
these admission steps.

## Application action and boundaries

Until a fixed package is actually published, retain a reviewed local patch
or restrict access to the affected Nexus administration surface. After
publication, update the affected dependencies and lockfile, rebuild and redeploy
the application; a GitHub merge alone does not update a running service.
Keep authentication, authorization, CSRF protection and CSP in place. If
unexpected administrative markup or actions have been observed, investigate
the application's own stored data, access logs and sessions.

Nexus should display stored text literally. Applications that depended on
entity-prefixed text injecting HTML into table cells must render any trusted
custom presentation through an explicitly reviewed application component.
The patch does not sanitize historical database contents or rotate credentials.
The existing 12.1.1 application-key migration requirements still apply.

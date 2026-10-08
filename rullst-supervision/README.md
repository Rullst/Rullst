# Rullst Supervision

Unpublished v13 implementation candidate. Optional exam-session observations and
parental application restrictions use explicit scoped authority and bounded local
SQLite state. The application owns authentication, current school membership,
resource authorization and independent guardian/reviewer verification.

`exam` provides typed sessions, explicit collection categories and browser/capture
observations. `parental` provides course/window policies. `sqlite` selects both
and the shared-local durable adapter. Optional `analysis` defines bounded camera
presence/audio activity adapters; combining it with `sqlite` adds authorization,
concurrency leases and deadline checks around invocation. There are no default
features or Core dependencies.

An application can opt into visibility, focus, clipboard occurrence and
fullscreen events. No clipboard contents, camera/audio model, media capture,
other-window inventory or device-wide control is included. An application can
supply a local model or external adapter. Observations never determine misconduct,
identity, attendance or grades.

Run the deterministic, explicitly simulated adapter example without credentials:

```bash
cargo run -p rullst-supervision --example observation_adapter --features sqlite,analysis
```

## Responsible use

**Maturity: Experimental** ([tiers](https://rullst.github.io/Rullst/book/maturity.html)).
This crate is designed for consented, transparent scenarios such as online
exams and parental controls over a learning application.

- Collection starts only after the learner acknowledges the exact policy,
  notice and selected categories, and only typed observations are stored.
- There is no covert capture mode, and the project will not add one.
- It is not designed for general security or CCTV surveillance, or for
  monitoring people who have not been told.
- Operators must comply with local law, especially data-protection and
  children's-privacy rules when minors are supervised. The crate provides no
  legal-compliance certification.

Read the full [Responsible use](https://rullst.github.io/Rullst/book/supervision.html#responsible-use)
section before you integrate it.

## Integration

The [observation integration guide](https://rullst.github.io/Rullst/book/supervision-observations.html)
explains selection, permission boundaries, adapter contracts and schema v2.
Unpublished schema v1 requires a separately reviewed transition to a fresh store;
opening it never upgrades, deletes or infers consent from existing records.

The [design and acceptance boundary](https://rullst.github.io/Rullst/book/supervision.html) records the
supported scope and its validation evidence. Source increments entered
`v13` through PRs #222 and #226; the latter's missing premerge archive gate was
repaired retrospectively, as recorded in the
[delivery evidence](https://rullst.github.io/Rullst/book/v13-delivery-plan.html). Final combined release
and package-admission requirements remain separate.
The standalone package is now included in the v13 release inventory. Initial
crates.io registration and final release acceptance remain outstanding; the
current `13.0.0-alpha.1` source is not a published stable release. Applications
opt in explicitly; no default framework dependency is added.

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

The generated LMS can opt into visibility, focus, clipboard occurrence and
fullscreen events. No clipboard contents, camera/audio model, media capture,
other-window inventory or device-wide control is included. An application can
supply a local model or external adapter. Observations never determine misconduct,
identity, attendance or grades.

Run the deterministic, explicitly simulated adapter example without credentials:

```bash
cargo run -p rullst-supervision --example observation_adapter --features sqlite,analysis
```

The [observation integration guide](../docs/src/supervision-observations.md)
explains selection, permission boundaries, adapter contracts and schema v2.
Unpublished schema v1 requires a separately reviewed transition to a fresh store;
opening it never upgrades, deletes or infers consent from existing records.

The [design and acceptance boundary](../docs/src/supervision.md) records the
supported scope, local generated-LMS/Chromium evidence and remaining archive,
full-workspace and hosted release requirements.
This package is not yet admitted for publication or production use.

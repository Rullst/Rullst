# Building an exam observation platform

**Unpublished v13 candidate.** `rullst-supervision` supplies scoped sessions,
selected collection categories, bounded observation storage and optional adapter
orchestration. It does not supply an entire assessment product or camera/audio
model. A developer supplies the exam UI, authenticated membership and enrollment,
media capture, model, reviewer workflow and accessibility alternatives.

## Choose and acknowledge exact categories

`exam::Collection` selects from `Capability`: visibility, window focus, clipboard
occurrence, fullscreen, camera/microphone/screen-share status, camera presence and
audio activity. `Collection::visibility_only()` is the default. Use
`ExamPolicy::with_collection` and `Acknowledgement::for_collection` with the same
set and policy/notice versions. Show that set before starting; an older or broader
acknowledgement cannot start/resume a differently configured session.

The stored `Session` exposes both the initial and current collection. A learner
can pause/end, or the host can call `restrict_collection` with the learner's
current authenticated context and revision to remove categories. This changes
the revision and fences pending results. Re-enabling a removed category requires
a new session with a new acknowledgement. Existing retained observations remain
available to the learner and separately authorized reviewers until expiry.
Browser permission and the legal basis selected by the application are separate
from this acknowledgement; selecting a capability proves neither.

## Browser observations

For the recognized full SQLite LMS, generate the optional consumer with:

```bash
cargo rullst make:supervision --supervision-source /path/to/rullst-supervision \
  --policy-version exam-v2 --notice-version notice-v2 \
  --retention-seconds 3600 --session-seconds 3600 \
  --browser-observations visibility,focus,clipboard,fullscreen
```

Omitting `--browser-observations` preserves visibility alone. The generated
notice and start/resume acknowledgement show and bind the selected categories.
The generator includes no camera/microphone permission prompt, analysis endpoint
or background media upload. Its ordinary session controls work without JavaScript.

| Observation | Meaning and limit |
| --- | --- |
| Page visible/hidden | This page's visibility changed; no inventory of other tabs. |
| Window focused/blurred | This window gained/lost focus; does not identify another window. |
| Copy/cut/paste attempt | An occurrence on this page; never clipboard contents or selected text. |
| Fullscreen entered/exited | Fullscreen changed; no forced fullscreen or integrity guarantee. |
| Capture started/stopped/denied/unavailable | Host-supplied client status for camera, microphone or screen share; no independent verification of capture. |

The collector ignores synthetic DOM events, but a user can modify browser code
or forge network reports: this is not authenticity or tamper protection. It
serializes one request at a time, waiting at least 1.1 seconds between requests,
with sixteen waiting occurrences at most. Queue overflow, network errors,
non-204 responses, page exit, form/session expiry or a session-control submission
stop collection and clear the queue. It does not automatically retry ambiguous
writes. A pause/end submission stops that page before the server confirms the
transition; other tabs are fenced by the committed server revision. Receipt time
is server reception time, not necessarily occurrence time.

Custom applications use `ObservationRequest::new` and `record_browser` or
`record_capture`. The request binds authenticated context, tenant/learner/resource,
session ID, current revision and the exact next sequence. Never accept `Context`
or its tenant/actor directly from an untrusted payload. The host refreshes access
before each operation. The store separately checks state, capability, revision,
sequence, rate, capacity and expiry transactionally. A page must not be able to
post an adapter finding through a browser-event route.

## Optional camera/audio adapters

Enable `analysis` plus `sqlite` to call `SqliteSupervision::analyze`. The
`analysis` feature alone exposes database-free, statically dispatched contracts:

- `Analyzer`: a bounded implementation returning a typed `Finding` and a fixed
  `AnalyzerDescriptor` with opaque ID, version, kind and simulation attribution.
- `AnalysisAuthorization`: application authorization refreshed before analysis
  and before recording, including current membership, entitlement and permission.
- `MediaSample`: borrowed JPEG/PNG bytes up to 1 MiB, or up to five seconds of
  mono 16 kHz signed 16-bit little-endian PCM (160,000 bytes). Signature/length
  checks do not decode, establish media validity or bind it to a learner.
- `AnalysisOptions`: a 1–15 second orchestration timeout. Simulated adapters are
  rejected unless `allow_simulated_for_testing()` is explicitly selected.

A local model is allowed; no paid provider is required. Implementations must
bound image dimensions/decoding, CPU, memory, network, retries and output, and
honor cancellation. A valid small compressed image can still require excessive
decoding resources. Run blocking or untrusted inference in an isolated worker
with enforceable limits; a Tokio timeout does not stop synchronous CPU work or
an external process. Do not log samples or credentials or include them in errors.
The framework stores only typed findings and source metadata, with no raw media.
The caller owns capture and transient sample lifetime and must stop/release its
media tracks when permission is revoked, paused or ended.

Camera results are person detected, no person detected or inconclusive. Audio
results are speech detected, no speech detected or inconclusive. A detector
failure returns `AnalysisUnavailable`; it must not become a negative observation.
No result identifies a person, speaker, emotion, intent, supplied answer or
misconduct. Missing reports never prove absence of an event. Source metadata
attributes the host's selected adapter; it is not a cryptographic attestation.

The SQLite orchestration checks authorization, then reserves one pending lease
per session across all processes before invoking the adapter. It releases the
SQLite transaction while analysis runs. Recording checks authorization again and
rechecks current session/revision/capability/sequence and the exact unexpired
lease. Pausing, ending, narrowing collection or expiring the lease fences the
result. Browser and analysis submissions share one sequence; the application
must coordinate producers, reload on conflicts and avoid replaying stale work.
A host membership check and the supervision database are separate transaction
boundaries, so the host owns atomic integration if its threat model requires it.

Cancellation/failure can leave a lease until its deadline (at most fifteen
seconds or session expiry). A timeout returns `UncertainCommit`: reload session
and observations before retrying, because a write might have committed. There
is no silent permission renewal, automatic scoring or eviction of active state.
The timeout requires a Tokio runtime with its timer driver enabled.

An executable offline example implements the traits with an explicitly simulated
inconclusive result and a disposable local store:

```bash
cargo run -p rullst-supervision --example observation_adapter --features sqlite,analysis
```

It uses no camera, microphone, network or third-party account. Replace its fixed
fixture authorization with real current application authorization. A simulated
pass does not establish model accuracy or provider interoperability.

## Storage transition and review

This unpublished extension uses SQLite schema v2. Opening v1 refuses it without
migrating its schema or data. Preserve old records under the existing retention
policy; quiesce the old preview and prepare a separately named private v2 store,
with a new independently retained deployment epoch and independently re-established
authority/restrictions. A fresh store has no parental enrollment, so provision
required restrictions before allowing protected learning. Do not delete old state
or change only its epoch to bypass refusal. There is no live upgrade tool yet.

`observations` returns at most 100 retained entries per page to the learner or a
currently authorized exam reviewer. `events` preserves the legacy visibility-only
view. Parental-management authority never grants observation review. The existing
rate, capacity, grant, retention, private-file, clock and rollback boundaries in
[the supervision contract](supervision.md) apply. Deleting records does not erase
WAL, free pages or backups.

## Acceptance evidence

Tests exercise real SQLite persistence, exact acknowledgement, disabled categories,
collection narrowing and stale revisions, separate source paths, all supported
browser/capture kinds, legacy visibility, v1 rejection, simulated adapters,
wrong-kind/failure handling, authorization revocation during analysis, shared
leases, timeout and no retained sample contents. Deterministic JavaScript tests
exercise the shipped collector's queue, sequence, selected events and stop/failure
behavior. The generated LMS/Chromium journey validates actual HTTP/security/forms
and browser behavior.

Local validation passed the five generator/process/composition contracts with
Chromium enabled, the generated LMS's fourteen original library tests and
production Clippy, 378 CLI unit tests, all-target strict Clippy for the CLI and
crate, seven isolated feature configurations, the database-free analysis
contracts and the executable offline example. The final focused sample caught
all nine selected authorization/restriction mutations, including a regression
for current-revision attempts against paused and ended sessions. This is a
bounded sample, not a whole-crate mutation score.

The extension passed hosted workspace checks in
[PR #226](https://github.com/Rullst/Rullst/pull/226). Its initially skipped archive
gate subsequently passed in
[exact-commit validation](https://github.com/Rullst/Rullst/actions/runs/35564765646).
The [delivery plan](v13-delivery-plan.md) records that retrospective repair.
The final release campaign remains required; no live provider/model test is claimed.

Browser boundaries follow [Page Visibility](https://developer.mozilla.org/en-US/docs/Web/API/Page_Visibility_API),
[focus events](https://developer.mozilla.org/en-US/docs/Web/API/Window/blur_event),
[copy events](https://developer.mozilla.org/en-US/docs/Web/API/Element/copy_event),
[camera/microphone permission](https://developer.mozilla.org/en-US/docs/Web/API/MediaDevices/getUserMedia)
and [screen capture](https://developer.mozilla.org/en-US/docs/Web/API/MediaDevices/getDisplayMedia).

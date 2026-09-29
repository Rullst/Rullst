# Transparent exam and parental supervision

**Status: unpublished v13 release candidate.** The implemented contract and
observation extension passed combined source/archive acceptance in PR #239. The
package is included in the release inventory, pending initial registration and
final release acceptance. It supplies no verified guardianship, device-wide
control or legal-compliance certification.

## Package and integration boundary

The separate crate has optional `exam`, `parental`, `sqlite` and `analysis` features. The
SQLite adapter selects both domain modules; neither the crate nor storage becomes
a Core/default dependency. Its first backend uses a concrete adapter generic
over a trusted server clock, with real SQLite and deterministic clocks in tests.
Applications integrate it directly. v13 removed the generated full-LMS consumer
(`make:supervision`) together with the complete Academy scaffold it targeted.

The host authenticates the actor and checks current school membership, course
access and administrative authority. Store context constructors validate opaque
identifiers; they do not authenticate a browser or prove a family relationship.
Host membership checks and supervision state are separate transaction boundaries.
They must run on every request; do not reuse a browser's tenant/role claim.

## Scoped authority

A trusted operator can install an expiring grant for one tenant, learner,
resource, delegate and action after independently verifying the relationship.
Provisioning includes a bounded opaque evidence reference; no identity documents
or raw verification evidence are stored. `ExamReview` and `ParentalManage` are
distinct. A parental manager cannot read exam observations through that grant.
Generic teacher roles, age results, matching email names and checkboxes confer
no such authority. No public self-provisioning route is planned.

Every delegate operation checks the current grant in its SQLite transaction.
Completed grant revocation blocks subsequent operations, including reads. It
cannot recall an already committed response. Revoking a parental manager does
not remove the learner's content restrictions; only the operator can remove
management explicitly. A persistent global revision counter prevents stale
forms from matching a recreated grant or policy after bounded cleanup.

## Visible exam sessions

The learner explicitly starts collection for a server-selected resource and
acknowledges exact policy/notice versions and the selected collection categories. Session IDs are random. The learner
can pause, resume with fresh acknowledgement or end collection. End is terminal;
every transition uses the displayed revision, and lifetime is at most eight
hours. Starting also compares the last retained session revision atomically;
a form from before a pause/end cannot silently start a new session. Generated
forms expire well before the minimum session retention. Session access is tenant/subject bound; existing course/assessment access
must still be checked by the application.

Visibility remains the default. Explicit selection also supports browser focus,
clipboard occurrence and fullscreen events, capture status and optional adapter
findings. Each observation binds a session,
its current revision and the exact next client sequence. Unknown, repeated,
out-of-order, paused, ended, expired and over-budget submissions are rejected.
Receipt time comes from the server; event rate and stored counts are bounded.
Events do not extend lifetime. They are incomplete, forgeable reports and never
establish cheating, attendance, attention, identity or time spent learning.
No event changes grades, answers, enrollment or penalties.

There is no camera, microphone, screen capture, key logging, clipboard content,
browsing history, location, fingerprint or arbitrary payload collection. The
consumer must show active/paused/ended state and stop sending after pause/end.
No-JavaScript controls must still work without claiming visibility observations.

## Observation extension candidate

The September 20 extension keeps the implementation in `rullst-supervision`.
There is no separate `rullst-proctoring` crate. It adds exact collection selection,
typed browser/capture observations and bounded camera-presence/audio-activity
adapter orchestration. The [integration guide](supervision-observations.md) covers
the APIs, source attribution, cancellation, schema transition and test boundaries.

The generated consumer observes only selected events on its own page. It cannot
enumerate every open window/application or recognize outside answers. The host
owns media capture permission, a local or remote model, accessibility alternatives
and human review; the framework includes no detector or automatic penalty.

## Parental application restrictions

An operator enrolls a learner and a server-selected access-policy resource into
parental management. Enrolled learners are denied while no valid policy exists.
An explicitly authorized manager can set a bounded course allowlist and one
absolute UTC access window, using the current revision. Missing/expired policy,
clock failure and storage failure deny protected operations. Unmanaged learners
retain the existing learning authorization. These restrictions cannot grant
enrollment, unpublished lessons or assessment permissions.

The generated integration must enforce policy on the original lesson-play and
progress paths. A new dashboard alone is insufficient. Existing independently
authorized cross-subject administrator corrections remain administrative work,
not learner access. They retain role, actor/subject membership and original
course checks; they confer no reviewer or parental-manager authority. The learner can inspect
the applicable window/policy. Already delivered content cannot be recalled.
There is no operating-system control, recurring timezone schedule or daily
screen-time accounting in this increment.

## SQLite, clocks and retention

Use one private local database with explicit exclusive initialization, verified
schema/configuration, WAL/full synchronization, bounded pool/lock waits and
parameterized SQL. `open` never creates or repairs missing state. The deployment
epoch must match the independently retained configuration. A trusted-directory
assumption is required; the adapter is not a sandbox for hostile filesystem
administrators or network filesystems.

`BEGIN IMMEDIATE` serializes authority reads, state changes and clock/revision
advancement. Check time after waits and before returning an authorized result.
Clock rollback, malformed records, capacity exhaustion and uncertain commits
fail closed. A failed or cancelled mutation can have an uncertain outcome;
reload state before retrying, and never infer success from a timeout.

Event retention is explicit, from one hour to seven days. Reads exclude expired
events immediately; bounded operator cleanup removes eligible rows. Session
metadata has bounded retention after its maximum lifetime. Grants and managed
learners have separate hard capacities; active management is never evicted.
SQL deletion does not erase copies in WAL, free pages or backups. Operators own
file permissions, encryption, keys, backups and retention beyond these records.
Restoring a stale database with its old epoch can restore revoked authority:
quiesce the feature and re-establish authority/policies with a new epoch before
resuming. No automatic rollback or failover claim is made.

## Acceptance before package admission

- Real database reopen and new-process operation, two independent pools, exact
  revision races, expiry during lock waits, cancellation, corruption, capacity,
  clock rollback and bounded retention/purge.
- Cross-school/learner/delegate negatives; expired/revoked grants; no role-only
  authority, no parental-to-exam privilege escalation and no stale-form replay.
- Real generated LMS authentication, CSRF, operator provisioning, session
  transitions and original learning-route enforcement; existing access denial
  remains effective for unmanaged and managed learners.
- Actual browser exercise with visible controls, keyboard/no-JS behavior,
  minimal event payloads and no capture or background reports after stop.
- Full workspace, strict Clippy, formatting, feature/panic boundaries and an
  installed-archive consumer before registry inclusion. Phase completion means
  the named behavior and tests exist; no mock-only package admission.

## Application integration

v13 no longer generates a supervision consumer: `make:supervision` targeted the
complete Academy LMS, which was replaced by a smaller starter. Integrate the
crate directly. The host application authenticates the actor, checks membership
and authority, provisions operators out of band and wires the SSR controls and
retention workers to the contracts above. Existing stores are never upgraded or
enabled by default. The removed generator remains in the git history and on the
stable v12 branch as a reference implementation.

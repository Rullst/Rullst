# Transparent exam and parental supervision

**Status: unpublished implementation in progress.** This contract defines the
first `rullst-supervision` increment. It is not an available production feature,
verified guardianship service, device controller or legal-compliance claim.

## Package and integration boundary

The separate crate has optional `exam`, `parental` and `sqlite` features. The
SQLite adapter selects both domain modules; neither the crate nor storage becomes
a Core/default dependency. Its first backend uses a concrete adapter generic
over a trusted server clock, with real SQLite and deterministic clocks in tests.
The first consumer is an explicit full-LMS SQLite opt-in. Other blueprints and
database topologies are outside this initial increment.

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
acknowledges exact policy/notice versions. Session IDs are random. The learner
can pause, resume with fresh acknowledgement or end collection. End is terminal;
every transition uses the displayed revision, and lifetime is at most eight
hours. Starting also compares the last retained session revision atomically;
a form from before a pause/end cannot silently start a new session. Generated
forms expire well before the minimum session retention. Session access is tenant/subject bound; existing course/assessment access
must still be checked by the application.

Only `PageVisible` and `PageHidden` events are accepted. Each binds a session,
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

## Generated preview being validated

The explicit command is available only with the matching local unpublished source:

```bash
cargo rullst make:supervision --supervision-source /path/to/rullst-supervision \
  --policy-version exam-v1 --notice-version notice-v1 \
  --retention-seconds 3600 --session-seconds 3600
```

It recognizes the complete SQLite LMS authentication and learning service before
planning atomic file changes. Custom authorization, other backends, existing
outputs or ambiguous CSRF routing require manual integration. It adds a shared
learning-service gate, scoped SSR controls, a local `supervision-admin` binary and
`SUPERVISION.md`; neither startup nor a browser can provision its own authority.
A separate nonzero form key binds authenticated cookie, actor, school, learner,
resource, policy/notice and revision. Forms expire after five minutes, shorter
than retained session metadata. An active/paused session is recovered when the
learner reopens its start page, rather than replaced by another session.

The browser collector sends only visibility changes, without automatic renewal
or heartbeats. It stops locally on pause/end submission, page exit, form/session
expiry or a failed report. Other open pages learn of revocation through rejection;
server state prevents acceptance after a completed pause/end. Own pause/end and
state reads remain possible after learning restrictions change, provided school
membership and lesson binding remain valid. New starts/resumes/reports require
current learning access.

Focused local acceptance now passes the real CLI/operator process, authenticated
HTTP journey and Chromium. The browser exercises keyboard start/pause/end with
JavaScript disabled, then real tab visibility, minimal request fields,
pause/resume/end and absence of capture or external page requests. HTTP negatives
cover missing CSRF, changed cookie, cross-subject/school access, duplicate/unknown
query fields, unknown/oversized bodies, stale session/policy forms, independent
authority/revocation and corrupted-store denial on original learning routes.
The public directly linked LMS compiles with privacy/age/supervision together;
both installation orders preserve application guidance and refreshed AI context.
The generated application also passes all fourteen original LMS library tests
and strict production Clippy, including the zero-panic lints. The added gate
preserves already-authorized administrative progress corrections while denying
restricted learner progress writes. All 376 CLI library tests pass locally.
Twenty-one crate tests pass locally. A focused mutation run of the changed
start/latest-session paths catches all six executable mutations; three attempted
`Default` replacements do not compile because sessions deliberately have no
`Default`. The earlier scoped-authority run caught twenty mutations with one
non-compiling replacement. These bounded samples are not a whole-crate mutation
score. Full workspace regression, the installed-archive rehearsal and hosted
release gates remain outstanding.

The distribution diagnostic audits and extracts the unpublished supervision
archive explicitly and feeds that extracted source to the installed CLI. Normal
release packaging now selects exactly `.github/release-order.json`, rather than
all workspace members. Candidate mode does not add supervision to publication
and must be removed when the package is admitted to that inventory.

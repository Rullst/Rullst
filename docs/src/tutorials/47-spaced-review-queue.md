# 47. Durable Spaced-Review Queue

The complete LMS blueprint can turn newly applied authoritative activity
scores into a durable due-review queue. This is useful for vocabulary, facts,
concept recall and other practice that benefits from repeated exposure.

## Generate the complete Academy starter

```bash
cargo rullst new language-academy --default --blueprint lms \
  --skip-initial-migration
cd language-academy
cargo test --offline --all-targets
```

Review scheduling belongs to the complete starter because it composes activity
evaluation, score persistence, school/course authorization and enrollment. The
smaller detached LMS profiles do not include this vertical.

## Enable a review policy

Each reviewable activity has one `activity_review_policies` row. Use the
generated Nexus model or application-owned parameterized administration code to
configure these fields:

| Field | Accepted contract |
|---|---|
| `algorithm_version` | Exactly `rullst-box-v1`. |
| `passing_ratio_milli` | 500–1000; `800` means 80%. |
| `first_interval_seconds` | One hour through 31 days. |
| `lapse_interval_seconds` | 60 seconds through the first interval. |
| `maximum_interval_seconds` | First interval through five years. |
| `enabled` | Exactly `1` to schedule; `0` disables scheduling. |

Malformed policy or existing state fails the score transaction closed. A
different algorithm version also requires an explicit application migration;
it is never silently reinterpreted.

## Submit an authoritative exercise

Use one of the authenticated activity routes described in the previous
tutorial:

- `POST /activities/{id}/attempts` for single choice;
- `POST /activities/{id}/attempts/matching` for bounded matching;
- `POST /activities/{id}/attempts/typed` for typed recall.

The route obtains identity, answer policy, points and time from server state.
When the score is new, the same database transaction updates
`activity_review_states` before it commits the score and outbox event. An exact
retry is a no-op and cannot make the interval grow twice.

`rullst-box-v1` applies a deliberately small inspectable transition:

- a passing result increments repetitions and schedules at least the configured
  first interval;
- a perfect result also raises the bounded ease value;
- later passes multiply the prior interval by that ease, capped by the policy;
- a lapse resets repetitions, increments lapses, lowers bounded ease and uses
  the configured lapse interval.

## Read the learner's due queue

After the normal authenticated session, request:

```http
GET /reviews/due?limit=20
```

The handler derives the learner and current time from server extensions; there
is no learner ID in the query. It accepts a limit from 1 through 50, checks the
active school membership and returns only activities whose course scope and
active enrollment still authorize the learner. Results are ordered by due time
and then activity ID.

```json
[
  {
    "activity_id": 42,
    "course_id": 3,
    "title": "Recall: ownership",
    "due_at_epoch": 1800086400,
    "repetitions": 1,
    "lapses": 0
  }
]
```

Render this response in the web application as the source of truth. An Omni
shell can present the same web-first route, but offline proposals must still be
reconciled with the server before the schedule is considered authoritative.

## Product and evidence boundary

The state includes learner/activity history and must participate in retention,
export and erasure policy. The generated foundation does not prove that its
intervals improve learning and does not implement FSRS, SM-2, speech
recognition, language-specific answer normalization, AI personalization,
streaks or a polished review UI. Tune policy only with reviewed product rules
and measured outcomes; use an explicit version plus migration for any new
algorithm.

Repository evidence covers deterministic pass/lapse transitions and a
materialized SQLite journey across single-choice, matching and typed recall,
including exact replay and cross-user denial. PostgreSQL/MySQL contention,
real-user efficacy, browser UX and physical Omni devices remain separate gates.

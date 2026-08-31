# 45. Accessible Academy Media

Rullst's LMS blueprint generates a bounded lesson-presentation foundation for
video and audio. It belongs to the web-first Academy slice: authorization and
progress remain server-owned, while the browser receives accessible media
markup and an escaped transcript.

## Generate the complete Academy starter

```bash
cargo rullst new language-academy --default --blueprint lms \
  --skip-initial-migration
cd language-academy
cargo test --offline --all-targets
```

The complete starter includes the integrated curriculum, assessment,
gamification, automation and notification journey. A smaller learning
foundation is available with `--lms-modules auth,learning`.

## Lesson media contract

Generated lessons store these fields:

| Field | Contract |
|---|---|
| `media_kind` | Closed renderer values: `video` or `audio`. |
| `media_url` | HTTPS URL or absolute same-origin path; control characters and backslashes are rejected. |
| `captions_url` | Required valid source for video; normally a same-origin `.vtt` path. |
| `transcript` | Required and bounded; the HTML renderer escapes it. |
| `language_tag` | Required bounded ASCII language tag such as `en` or `pt-BR`. |

For production, prefer application-owned same-origin or signed media. Put a
caption file at `static/media/lesson.pt-BR.vtt` and store the public source as
`/static/media/lesson.pt-BR.vtt`; `Server` mounts the local `static/` directory
at `/static`.

```text
WEBVTT

00:00.000 --> 00:04.000
Bem-vindo à primeira atividade.
```

The blueprint intentionally does not copy a media binary. Add your reviewed
audio/video asset or application-specific object-storage delivery, then use a
same-origin path such as `/static/media/lesson.webm`. If you choose a remote
host, add only that reviewed origin to the application's `media-src` CSP; do
not weaken the policy to arbitrary HTTPS.

## What the generated player enforces

- no autoplay;
- native video/audio controls;
- a caption track for every video;
- an always-available transcript for video and audio;
- visible keyboard focus and nonce-bound styles;
- escaped title and transcript values;
- fail-closed rendering for unknown kinds, insecure sources or invalid
  accessibility metadata.

The protected lesson controller still checks the authenticated learner's
school, enrollment, entitlement and release policy before rendering the player.
Progress submissions use CSRF and idempotency data and remain authoritative in
the database.

## Evidence boundary

Repository tests materialize the generated SQLite project and exercise both
successful renderers and negative source/metadata cases. They do not prove
codec support, buffering behavior, screen-reader quality, subtitle accuracy,
microphone or speech recognition, physical mobile devices, CDN delivery or app
store behavior. Run browser accessibility tests with your real content and
deployment before making those claims.

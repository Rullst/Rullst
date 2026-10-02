# 45. Accessible Academy Media

Rullst's LMS blueprint generates a bounded lesson-presentation foundation for
video and audio. Authorization and progress remain server-owned, while the
browser receives accessible media markup and an escaped transcript.

## Generate the LMS starter

```bash
cargo rullst new language-academy --default --blueprint lms \
  --skip-initial-migration
cd language-academy
cargo test --all-targets
```

The starter includes the course catalog, modules, lessons, this accessible
player, enrollment, progress, login and a Nexus admin. Earlier releases also
generated a complete Academy (assessment, gamification, automation and
notifications); v13 removed it in favor of this smaller starter.

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

The blueprint intentionally does not copy a media binary. Its four seeded
development lessons point at remote sample video/audio files, and its two
video lessons use the generated `static/media/memory-safety.en.vtt` and
`static/media/first-project.en.vtt` captions. The default CSP
(`default-src 'self'`) does not allow that remote media, so replace the seeded
lessons in Nexus (`/nexus`) with your own content. Add your reviewed
audio/video asset or application-specific object-storage delivery, then use a
same-origin path such as `/static/media/lesson.webm`. If you choose a remote
host, add only that reviewed origin to the application's `media-src` CSP; do
not weaken the policy to arbitrary HTTPS. Core also defaults COEP to
`require-corp`, so the remote media server must emit a compatible
`Cross-Origin-Resource-Policy` response. When a reviewed server cannot do so,
an application can explicitly set `coep = "credentialless"` under `[security]`
in `Rullst.toml`; browsers then omit credentials for eligible cross-origin
no-CORS requests. Prefer same-origin delivery, and test the exact CSP/COEP/media
combination in the deployed browser rather than disabling isolation globally.

## What the generated player enforces

- no autoplay;
- native video/audio controls;
- a caption track for every video;
- an always-available transcript for video and audio;
- visible keyboard focus and nonce-bound styles;
- escaped title and transcript values;
- fail-closed rendering for unknown kinds, insecure sources or invalid
  accessibility metadata.

The protected lesson controller checks the authenticated learner's
active enrollment before rendering the player.
Progress submissions use CSRF and idempotency data and remain authoritative in
the database. Each rendered player carries a fresh random key, scoped to the
requested percentage on submission: resubmitting the same click replays the
stored event, while a later save, or another button, records new progress.

## Evidence boundary

Repository tests check the generated player source for its caption track,
native controls, source and transcript bounds, and the absence of autoplay,
inline styles and `hx-` attributes. They also materialize the generated SQLite
project and run its own tests (catalog query bounds, progress keys and the
learning service's owner boundary); no generated test renders the player or
its rejected source/metadata cases. They do not prove codec support, buffering
behavior, screen-reader quality, subtitle accuracy, microphone or speech
recognition, physical mobile devices, CDN delivery or app store behavior. Run
browser accessibility tests with your real content and deployment before making
those claims.

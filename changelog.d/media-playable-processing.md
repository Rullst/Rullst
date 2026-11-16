### Changed

- **Breaking:** `rullst-media`'s `Processing` is `#[non_exhaustive]` and has a
  new `Playable` state. Bunny's API status 8 (`JitPlaylistsCreated`, from
  just-in-time encoding with Premium Encoding) now maps to `Playable` instead
  of `Processing`: the playlists exist, so Bunny's player streams the video
  while the remaining encoding goes on. Status 7 (`JitSegmenting`) stays
  `Processing`, since the first segments do not exist yet, and `Ready` still
  means status 4 (`Finished`). `MediaService` publishes a `Playable` video and
  grants it only `PlaybackKind::Embed`, because Bunny supports just-in-time
  playback only in its own player; `Hls` and `Mp4_720p` return
  `MediaError::Unsupported` until the video is `Ready`. A published video
  whose next read is neither `Playable` nor `Ready` is withdrawn, as before.
  Webhook status codes are a separate numbering and still only trigger a
  refresh. New `Processing::is_playable()` and `Processing::allows(kind)` tell
  an application whether a video can be watched now and with which kind. A
  `match` on `Processing` outside `rullst-media` needs a wildcard arm;
  `cargo rullst upgrade` reports each one (`V13-MEDIA-PROCESSING-MATCH`).
  Reported by the plataforma application.

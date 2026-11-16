### Changed

- The standalone Studio's project list (`projects.json`) is forward
  compatible. A Studio keeps the fields it does not know in every object and
  writes them back unchanged when it saves, so an older 13.x Studio no longer
  drops what a newer one stored; they are bounded to 32 fields per object,
  names of 64 bytes and 16 KiB of JSON per object. Names, base URLs, alert
  thresholds and notification settings are still validated as when they are
  saved. The proxy setting is checked when it is used: a proxy mode of a
  newer Studio is kept and blocks every connection until another is chosen.
  `version` now counts incompatible changes only: a later 13.x adds fields
  and never raises it, and a file with a higher version is refused without
  being changed, with the new `ProjectError::NewerFormat`, whose message
  names the Studio that reads it (from the file's `min_studio_version`).
  `AlertRules` deserialization ignores unknown fields. A Studio older than
  this release still refuses the file once a setting new in this release is
  saved (desktop notifications, a proxy, a p99 or security-events alert
  rule); update every Studio that shares the file first.

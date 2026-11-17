### Changed

- `rullst-nexus` and `rullst-studio` no longer depend on `chrono`. Nexus reads
  date, time, date-time and UTC form values with `jiff` 1.0 and binds the new
  `rullst_orm` types (`RullstValue::CivilDate`, `CivilTime`, `CivilDateTime`
  and `Timestamp`), which every driver receives exactly as it received the
  `chrono` values before; its `tz` feature moves from `jiff` 0.2 to 1.0. Forms
  accept and refuse the same values, store the same text and show the same
  inputs in the administrator's time zone, including years 0 and 10000 UTC
  that an offset reaches from 0001 and 9999: an instant after
  9999-12-30T22:00Z, which a `jiff` timestamp cannot hold, is bound as the
  RFC 3339 text the `Any` pool already sent for it. Studio's request log keeps
  its `2026-10-15T09:30:00.500+00:00` timestamps.

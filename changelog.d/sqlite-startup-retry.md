### Fixed

- When another process kept the file locked past the 5-second wait, the
  `initialize` and `open` of the Labs, Media, Supervision and Privacy SQLite
  stores could end with SQLite's busy result instead of a retry: on Linux
  SQLite's lock wait can run out before the equal pool wait, and only a pool
  timeout was tried again. Both are now tried again, three attempts in total,
  on every platform, and a step whose wait runs out at every attempt fails with
  a new typed error that only these calls return:
  `LabError::StorageTimeout`, `MediaError::StorageTimeout`,
  `SupervisionError::StorageTimeout` (also from `upgrade_event_schema`),
  `ConsentError::StoreTimeout` or `AgeError::StoreTimeout`, instead of
  `Storage`, `Busy` or `StoreUnavailable`. Nothing was changed, so startup can
  be tried again later.
- `SqliteBroker::connect` and its variants had no outer deadline and no retry:
  a locked file or a stalled disk kept them waiting on SQLite's 30-second
  waits. Each opening attempt now has a 10-second deadline and is tried again,
  three attempts in total, through the same helper as `WebhookOutbox::open`;
  when every attempt expires they fail with the new
  `MessagingError::StorageTimeout { operation: "connect" }`.
- The start-up retry tests of these stores and of `WebhookOutbox::open` ran on
  Windows only. They now run on every platform with the waits shortened to
  fractions of a second, holding the file under SQLite's exclusive lock past
  the wait.

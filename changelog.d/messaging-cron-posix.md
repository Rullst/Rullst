### Changed

- **Breaking:** Messaging's recurring schedules (`schedules-postgres`) read
  new cron expressions as POSIX five-field cron, exactly as Core's
  `Scheduler::task` does: weekdays 0-7 (0 and 7 are Sunday; it was 1 for
  Sunday to 7 for Saturday), a day matching *either* day field when both are
  restricted (both had to match), and each field valid on its own. Every
  `RecurringDefinition::new` builds a POSIX definition, stored with a
  `"syntax":"posix"` marker; an expression that only the old grammar accepts,
  such as `1, 2 * * *`, fails with `InvalidInput("POSIX five-field UTC cron;
  this expression uses the legacy syntax")`. Definitions stored earlier have
  no marker and keep the old grammar (`CronSyntax::Legacy`, deprecated,
  removal planned for 14.0), so their occurrences never change. The new
  `CronSyntax` enum, `RecurringDefinition::syntax` and
  `RecurringMetadata::syntax` (from `create` and `schedules`) tell them apart,
  and `RecurringDefinition::posix_equivalent` gives the POSIX expression with
  exactly a legacy definition's occurrences (`0 9 * * 1` gives `0 9 * * 0`),
  or `None` when there is none (both day fields restricted, for example).
  `PostgresRecurringStore::create` keeps a stored legacy definition: one with
  the same expression and fields replays it, still legacy, with a `tracing`
  warning; any other difference fails with the new
  `RecurringError::LegacyDefinition` (cancel it and create the POSIX
  definition under a new name; a cancelled name is never reused). An earlier
  13.0 prerelease cannot read definitions written with the marker. The `cron`
  crate and chrono are no longer dependencies: legacy expressions run on a
  port of the crate's grammar checked against golden vectors recorded from
  it.

### Changed

- **Breaking:** `Scheduler::task`, `task_in_tz`, `deferrable_task` and
  `deferrable_task_in_tz` evaluate cron expressions with Core's own
  five-field evaluator instead of the `cron` crate, which leaves the
  dependency graph (9 fewer packages in the Blank and API starters, compiled
  and in `Cargo.lock`). The syntax, the day-of-month or day-of-week rule,
  time zones and occurrences from 1970 through 2100 are unchanged, checked
  against the `cron` crate by property tests. Each field must now be valid
  on its own: the `cron` crate joined a field that starts or ends with `,`,
  `-` or `/` with its neighbour, so `1, 2 * * *` ran at minutes 1 and 2 of
  every hour from January to July only. Such an expression now fails with
  `SchedulerError::InvalidCron`, whose messages now name the field.

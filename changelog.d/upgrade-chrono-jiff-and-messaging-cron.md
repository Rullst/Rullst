### Added

- `cargo rullst upgrade` reports where an application's chrono values meet
  the Rullst APIs that take jiff values since 13.0.0-alpha.6, each as a
  must-change finding on the "`chrono` replaced by `jiff`" row of the
  migration guide: `V13-ORM-CHRONO-TYPES`, a chrono type (`DateTime<Tz>`,
  `NaiveDateTime`, `NaiveDate`, `NaiveTime`, also inside `Option`, another
  generic, a type alias or a `use … as` rename) in a field of a
  `#[derive(Orm)]` or `#[derive(Nexus)]` model, a `rullst::orm::chrono`,
  `rullst::db::chrono` or `rullst_orm::chrono` path, `sqlx::types::chrono`
  or a bare `chrono::` path that only Rullst's dependencies made available,
  `RullstValue`'s chrono variants, and a chrono value passed to
  `RullstValue::from` or a query method; `V13-LOCALE-JIFF-DATES`,
  `Locale::format_date`, `format_time` and `format_datetime` calls;
  `V13-MAIL-JIFF-TIMESTAMPS`, Mail's `send_at`, `with_scheduled_at` and the
  `expires_at` of `PasswordResetAt` and `EmailLoginAt`; and
  `V13-CAPITAL-INVOICE-JIFF-DATE`, Capital's `Invoice::new`. An
  application's own chrono code is not reported.
- `V13-MESSAGING-CRON-POSIX` reports each `RecurringDefinition::new` call
  for review, now that messaging schedules read cron expressions with the
  scheduler's POSIX syntax, except a string literal that means the same
  under both syntaxes. A literal's finding gives its POSIX form for a
  schedule not yet stored (weekday numbers moved down by one) or why there is
  none (both day fields restricted, a field the old syntax joined with its
  neighbour or split, a form the new syntax refuses).
- A finding can carry a note for its location: the plan prints it as
  `line N: …` under the finding, the Markdown report as a sub-item, and the
  `rullst.upgrade-plan.v1` JSON adds a `note` key to each source finding
  (`null` when there is none).

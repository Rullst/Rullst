### Added

- `rullst_orm::{Timestamp, DateTime, Date}` have arithmetic that returns the
  ORM types instead of jiff's: `checked_add`, `checked_sub`, `saturating_add`,
  `saturating_sub` (with a `jiff::SignedDuration`, a `jiff::Span` or a
  `std::time::Duration`, as jiff accepts), `duration_until` and
  `duration_since` (taking the ORM type or jiff's), and `tomorrow`/`yesterday`
  on `DateTime` and `Date`. These methods replace the jiff methods of the same
  names that `Deref` reached before. There are no `+`/`-` operators, which
  would panic on overflow. `Date::days_until` and `Date::days_since` count
  whole days.
- `DateTime::date()` and `DateTime::time()` return `rullst_orm::Date` and
  `rullst_orm::Time`.
- `Timestamp::to_rfc3339()` writes the same text as chrono's
  `DateTime<Utc>::to_rfc3339()`: a `+00:00` offset and 0, 3, 6 or 9 fraction
  digits (`2026-10-15T09:30:00.500+00:00`).
- A reference to any of the four types converts into its jiff type
  (`From<&rullst_orm::Date> for jiff::civil::Date`, and so on), so
  `locale.format_date(&model.due_on, style)` compiles.

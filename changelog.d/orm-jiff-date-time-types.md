### Added

- `rullst-orm` has its own date/time types for model fields and query values,
  built on `jiff` 1.0 (re-exported as `rullst_orm::jiff`, so applications use
  the same version): `rullst_orm::Timestamp` (a UTC instant, for
  `timestamp_tz()` columns), `DateTime` (a date and time without a zone, for
  `timestamp()`), `Date` and `Time`, also reachable as `rullst::orm::…`. Each
  wraps the jiff type of the same name, dereferences to it and converts to and
  from it (results of jiff arithmetic convert back with `into()`), and
  `Timestamp::now()` reads the clock. They replace the `chrono` types
  `DateTime<Utc>`, `NaiveDateTime`, `NaiveDate` and `NaiveTime` without a
  visible change: they bind and decode on SQLite, PostgreSQL and MySQL/MariaDB
  with the same bytes on every driver, the same SQLite text and the same
  MariaDB `DATETIME` parameter for a UTC instant in generated writes, filters
  and `Orm::raw`, and read every stored form the `chrono` decoders read, so
  old and new rows mix in one column; `Display`, `Debug` and serde write
  chrono's text (`2026-10-15 09:30:00.500 UTC`, `2026-10-15T09:30:00.500Z`),
  `FromStr` and serde read every text chrono read (and jiff's forms), and
  `Default` is chrono's (the Unix epoch, 1970-01-01, midnight). Reading
  differs where jiff's range ends: `DateTime`, `Date` and `Time` hold years
  -9999 to 9999 and `Timestamp` -9999-01-02T01:59:59Z to
  9999-12-30T22:00:00.999999999Z (chrono read years up to ±262,142), so a
  value outside is a decode or parse error, never clamped; this includes the
  end-of-time sentinel `9999-12-31 23:59:59` in a PostgreSQL `timestamptz` or
  MySQL/MariaDB `DATETIME` read as a `Timestamp` (find such rows with
  `WHERE col > '9999-12-30 22:00:00+00'` on PostgreSQL or
  `WHERE col > '9999-12-30 22:00:00'` on MySQL/MariaDB; a `timestamp` column
  read as `DateTime` keeps the full range). A stored leap second
  (`23:59:60.5`) reads as `23:59:59.5`; PostgreSQL `infinity`/`-infinity` is a
  decode error where chrono panicked; and a MySQL/MariaDB zero or truncated
  binary date/time value is an error where chrono's decoder could panic.
  `RullstValue` gains the `Timestamp`, `CivilDateTime`, `CivilDate` and
  `CivilTime` variants, and `#[derive(Nexus)]` gives these fields the
  `DateTimeUtc`, `DateTime`, `Date` and `Time` widgets by type, so an
  application type named `Date` or `Timestamp` stays a text field. Because
  `rullst::db::*` now also exports `DateTime`, `Date`, `Time` and `Timestamp`,
  combining it with `use chrono::prelude::*` makes `DateTime` ambiguous;
  import one by name. The `chrono` types keep working until they are removed
  before 13.0.0-alpha.6 is published.

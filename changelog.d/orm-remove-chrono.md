### Removed

- **Breaking:** `rullst-orm` no longer depends on `chrono` (RUSTSEC-2026-0543,
  unmaintained). The `rullst_orm::chrono` (`rullst::orm::chrono`) re-export, the
  `RullstValue::DateTime`, `NaiveDateTime`, `Date` and `Time` variants and the
  SQLx codecs for `chrono` model fields are gone: use `rullst_orm::Timestamp`,
  `DateTime`, `Date` and `Time` (and `RullstValue::Timestamp`,
  `CivilDateTime`, `CivilDate`, `CivilTime`), which store, read, format and
  serialize as the `chrono` types did, and `rullst_orm::jiff` for date
  arithmetic. `#[derive(Nexus)]` no longer infers widgets from a field's
  `chrono` spelling; the ORM's date/time types get theirs by type. Redis
  query-cache keys of date/time values are unchanged.

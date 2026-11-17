### Changed

- **Breaking:** Core, Mail and Capital use jiff 1.0 instead of chrono for
  instants and calendar dates
  ([RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543.html):
  chrono is no longer maintained). The changed signatures:
  `Locale::format_date(&self, impl Into<jiff::civil::Date>, DateStyle)`,
  `format_time(&self, impl Into<jiff::civil::Time>, TimeStyle)` and
  `format_datetime(&self, impl Into<jiff::civil::DateTime>, DateStyle,
  TimeStyle)` (they took `&impl chrono::Datelike`, `&impl chrono::Timelike` and
  a reference to a value of both; pass civil values by value or a
  `&jiff::ZonedDateTime`); Mail's `Message::send_at` field
  (`Option<jiff::Timestamp>`) and method (`send_at(jiff::Timestamp)`), the
  memory driver's `with_scheduled_at(jiff::Timestamp)` and the `expires_at:
  jiff::Timestamp` of `AccountEvent::PasswordResetAt` and
  `AccountEvent::EmailLoginAt`; Capital's `Invoice::date` field and
  `Invoice::new(invoice_id, customer_email, date: jiff::Timestamp, currency)`.
  Replace `chrono::Utc::now()` with `jiff::Timestamp::now()`, `Utc::now() +
  Duration::minutes(5)` with `Timestamp::now() +
  jiff::SignedDuration::from_mins(5)` (or `checked_add`) and `.timestamp()` with
  `.as_second()`. What users, peers and storage see is unchanged: queued mail
  and invoice JSON keep chrono's RFC 3339 form (`2026-10-22T15:45:30.250Z`, 0, 3,
  6 or 9 fractional digits) and read what chrono wrote, invoices print the
  same `YYYY-MM-DD` date, the Resend `scheduled_at` field, the account-mail
  expiry line and the log driver's lines keep their `+00:00` (or local) offset,
  and security event times keep three fractional digits. Core's scheduler
  computes the same occurrences, daylight-saving rules included: the cron
  differential tests now check golden vectors recorded from the `cron` crate,
  which is no longer a dev-dependency. jiff becomes a non-optional Core
  dependency (`scheduler-tz` still adds the time-zone database), and Core, Mail
  and Capital no longer depend on chrono. `examples/mail-postmark-driver` uses
  jiff too, with the same webhook event identifiers.

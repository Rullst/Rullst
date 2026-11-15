### Changed

- SQLite-only applications compile no TLS stack. The Blank and API starters
  with SQLite went from 374 to 318 packages in `Cargo.lock` and from 297 to
  276 compiled third-party packages, with no rustls, aws-lc-rs, aws-lc-sys or
  webpki roots; the Blank starter without a database also dropped to 318.
  `rullst-orm` enables SQLx's `tls-rustls-aws-lc-rs` only with its PostgreSQL
  and MySQL drivers (`strict-postgres`, `strict-mysql`, `drivers-all`,
  `pgvector`), Core's `orm` and `strict-sqlite` no longer build rustls, and
  newly generated SQLite projects declare `sqlx` without
  `tls-rustls-aws-lc-rs`. A project generated earlier keeps its TLS stack
  until it removes that feature from a SQLite-only `sqlx` dependency.
- `Server` start-up still installs aws-lc-rs as the rustls process default
  wherever TLS is used, through Core's new `tls` feature: Core's
  `strict-postgres`, `strict-mysql`, `drivers-all`, `storage-s3` and
  `telemetry`, the `rullst-ai`, `rullst-capital` and `rullst-mail` crates,
  the OAuth and PostgreSQL features of `rullst-auth` and the facade's other
  HTTPS, Redis and PostgreSQL features enable it, and CI checks every facade
  feature. The facade's new `tls` feature enables it for an application whose
  own code makes rustls connections and relied on `orm` installing that
  default. `rullst-connect`'s `postgres` and `mysql` features now enable
  SQLx's TLS themselves.
- Studio's revenue dashboard (`/studio/capital`, `/api/revenue`,
  `rullst_studio::revenue_dashboard`) and its `rullst-capital` dependency are
  behind Studio's new `capital` feature, on by default in `rullst-studio` and
  enabled by the facade's `capital` together with `studio` (the SaaS starter
  has both). Without it `/studio/capital` says how to enable it and
  `/api/revenue` answers 404.

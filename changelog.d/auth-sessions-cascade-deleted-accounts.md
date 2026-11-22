### Fixed

- The `rullst_auth_sessions` table that `cargo rullst auth` and the SaaS
  starter create had no foreign key on `user_id`, because the session
  registry does not know the application's accounts table, so an account
  deleted with SQL (a retention job, a deletion request) left its sessions,
  with the browser family and times, until they expired (30 days by
  default). The generated `*_create_rullst_auth_sessions_table` migration now
  links `user_id` to `users (id)` with `ON DELETE CASCADE`: on SQLite it
  creates the registry's table with the key before
  `SessionRegistry::create_table` adds the indexes, and on PostgreSQL and
  MySQL/MariaDB it adds the constraint after. `cargo rullst auth` gives the
  migration a later stamp than the users migration it writes, so that it
  runs after it. `SessionRegistry::create_table`, `open`, the session
  management guide and the alpha.5 to alpha.6 checklist describe the case and
  how an existing application adds the key (#827).

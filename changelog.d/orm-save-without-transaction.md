### Changed

- With the query timeout off (`Orm::set_query_timeout(0)`; it is 30 seconds
  by default), a direct `save()` that sends one statement no longer wraps it
  in `BEGIN`/`COMMIT`: an insert, a full-row update, or a tracked update that
  writes every column of a model without `#[orm(db_default)]` columns or a
  soft-delete marker, for a model that is not auditable and has no
  `before_save`/`after_save` hook, policy or registered observer. The save
  sends one statement to the database instead of three. A rejected statement
  still persists nothing and leaves the handle as before; every other save,
  and every save while a query timeout is set, keeps its transaction. Two
  outcomes differ from a transaction: an insert whose `id` column does not
  match the model's `i32` key keeps the row when `save()` returns the decode
  error, and a save whose future is dropped after its statement was sent may
  still persist, as a dropped `COMMIT` may.
- A tracked update that writes every column of a model without
  `#[orm(db_default)]` columns or a soft-delete marker no longer reads the row
  back (`RETURNING *` on PostgreSQL and SQLite, a `SELECT` on MySQL/MariaDB),
  whatever the query timeout.

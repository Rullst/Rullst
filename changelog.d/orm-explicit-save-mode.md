### Changed

- **Breaking:** every `#[derive(Orm)]` and `#[derive(TursoModel)]` model now
  declares what `save()` writes to a stored row, and a model that does not
  choose no longer compiles. Until now a model without a
  `#[sqlx(skip)] snapshot: RowSnapshot` field silently got full-row updates,
  so two requests that changed different fields of one row lost one of the
  changes (a student's XP award vanished when a profile edit loaded before it
  was saved). Declare the field to write only the changed columns
  (recommended, and what the CLI generates), or `#[orm(full_row_save)]` on the
  struct to keep writing every column; the compile error shows both lines, and
  a model with both fails too. Turso-primary models always write every
  column: they declare `#[orm(full_row_save)]`, and a `RowSnapshot` field on
  them, which they ignored, no longer compiles. `cargo rullst upgrade` reports
  each model without a valid choice at its struct name (`V13-ORM-SAVE-MODE`).
- The Turso models that `cargo rullst make:model`, `make:chat-session` and the
  Turso Blank starter write declare `#[orm(full_row_save)]`, and
  `generate:models` names the dirty-tracking field `row_snapshot` for a table
  with a `snapshot` column instead of generating a full-row model. The ORM
  examples and the blog example declare a `RowSnapshot` field.

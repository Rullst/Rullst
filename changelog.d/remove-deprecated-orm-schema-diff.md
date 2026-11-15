### Removed

- **Breaking:** `rullst_orm::schema_diff` (`extract_tables_from_ast`,
  `ParsedTable` and `ParsedField`), deprecated in 13.0.0-alpha.5, was removed
  with the `syn` and `walkdir` dependencies it alone used, so applications
  with the ORM no longer build them for it. Nothing in Rullst called it, and
  it has no library replacement: write a migration from the models with
  `cargo rullst make:migration:auto`, or read their tables and columns with
  `cargo rullst inspect schema --json`. `cargo rullst upgrade` reports each
  use (`V13-ORM-SCHEMA-DIFF-REMOVED`).

### Security

- `GraphQuery::read_only` (SurrealDB graph queries in `rullst-orm`) now
  accepts only an allowlisted read grammar: `MATCH` paths of nodes and edges,
  an optional `WHERE` of comparisons and `IS [NOT] NULL` tests joined with
  `AND`, `OR` and `NOT`, `RETURN` items with optional `AS` aliases and an
  optional `ORDER BY` with `ASC` or `DESC`. It used to refuse only the
  `INSERT`, `SET`, `REMOVE`, `DELETE` and `LIMIT` tokens, so a write clause
  that a later SurrealDB release implements (`CREATE`, `MERGE`, `CALL`, …)
  would have passed the read-only check. Every other clause, function or
  procedure call, comment, parameter, escaped string and caller-provided
  `LIMIT` is now refused with `PolyglotError::InvalidIdentifier`; a read
  query outside the grammar, such as `OPTIONAL MATCH` or `DISTINCT`, must be
  rewritten in it. A keyword inside a string literal no longer causes a false
  refusal.

### Changed

- A SQLite pool created by `Orm::init`, `Orm::init_with_options` or
  `Orm::init_with_replicas` no longer pings each connection before handing
  it out (SQLx's `test_before_acquire`). The connection is a file the
  process opened itself, and the round trip to its worker thread cost about
  as much as a primary-key query. A connection that failed still fails its
  next query, and SQLx then closes it. PostgreSQL and MySQL pools keep the
  check.

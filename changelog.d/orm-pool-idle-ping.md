### Changed

- A PostgreSQL or MySQL pool created by `Orm::init`, `Orm::init_with_config`,
  `Orm::init_with_options` or `Orm::init_with_replicas` now pings a
  connection before handing it out only when the connection has been idle
  for at least 500 ms, instead of before every acquisition (SQLx's
  `test_before_acquire` is off). A connection that fails that ping is closed
  and replaced. The ping when a connection returns to the pool is unchanged,
  so a one-statement request takes two round trips instead of three. A
  connection that dies less than 500 ms after its release fails its next
  query, and SQLx then closes it. SQLite pools still never ping before
  handing out a connection.

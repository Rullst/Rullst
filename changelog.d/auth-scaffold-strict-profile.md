### Changed

- `cargo rullst auth` now needs exactly one strict database profile
  (`strict-sqlite`, `strict-postgres` or `strict-mysql`, as
  `cargo rullst new --database ...` selects) and enables its session registry
  feature (`auth-session-registry-sqlite`, `-postgres` or `-mysql`). A project
  that selects `drivers-all` or several profiles is refused before anything
  is written. In the SaaS starter and new `cargo rullst auth` scaffolds,
  `auth_middleware` inserts the signed-in account's `AuthenticatedSession`
  beside its id, `POST /password/change` reads it instead of decoding the
  cookie again, and a storage failure during sign-out answers 503 and keeps
  the cookie instead of clearing it.

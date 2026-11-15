### Added

- `RULLST_ACCESS_LOG`, in the process environment or `.env`, chooses which
  requests `Server` writes to the access log: `on` (the default), `errors`
  (responses with a status of 400 or above), `1/N` (those and one in `N` of
  the others, `N` from 1 to 1000000) or `off`, which leaves the access log out
  of the stack. The health-probe and path rules still apply, and any other
  value stops startup with `ServerError::Configuration`.

### Fixed

- `LocalStudioAccess::protect_router` and the loopback `NexusAuthPolicy` checked the
  environment at build time from the process `RULLST_ENV` and `APP_ENV` only, while their
  per-request checks also honoured `.env` and `[app] env`. An application that sets
  `RULLST_ENV=production` in `.env` or `Rullst.toml` could therefore build the
  credential-free loopback policy in a debug build. Both now resolve the environment as
  `Server` does, and refuse for staging, production, an invalid value or an unreadable
  `.env` or `Rullst.toml`.

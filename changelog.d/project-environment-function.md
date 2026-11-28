### Added

- `rullst::config::project_environment()` resolves the runtime environment of the working
  directory as `Server` does (process `RULLST_ENV`, then `APP_ENV`, then either one from
  `.env`, then `[app] env`) without a `Server`, for code that decides while building a router.

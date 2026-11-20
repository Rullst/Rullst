### Fixed

- In staging and production, the `Mail` facade no longer reports success for
  mail it never sends when the selected driver's credential is missing. An
  empty or unset `RESEND_API_KEY`, `SENDPULSE_API_KEY`, SES proxy token
  (`AWS_SES_TOKEN`/`AWS_SES_BEARER_TOKEN` when no AWS keys select native SES)
  or `MAIL_HOST` selected the offline mock, which captured every message
  (password resets included) and returned `Ok`, with one warning per process.
  It now returns `MailError::ConfigError` naming the setting, so a worker fails
  the job. This changes behaviour only for a misconfigured staging or
  production application: set the credential, or an explicit `mock_*` value
  to keep the offline mock on purpose. Development and test keep the
  empty-credential fallback, and drivers built with `try_new` are unchanged.
  The environment is read as `Server` reads it, now including `[app] env`
  from `Rullst.toml` when `MAIL_DRIVER` and `MAIL_FROM` are both set.

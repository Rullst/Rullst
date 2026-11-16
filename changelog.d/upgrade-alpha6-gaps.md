### Added

- `cargo rullst upgrade` reports more of the 13.0.0-alpha.6 changes that
  earlier only the alpha.5 to alpha.6 checklist named, each as a review
  finding: `V13-SCHEDULER-CRON-FIELDS`, a string literal passed to
  `Scheduler::task`, `task_in_tz`, `deferrable_task` or
  `deferrable_task_in_tz` that Core's new cron evaluator refuses because a
  field starts or ends with `,`, `-` or `/` (the rule runs the evaluator, so
  only refused expressions are reported); `V13-CSRF-TOKEN-COMPARED`, code and
  tests that compare a CSRF token with the `rullst_csrf` cookie's value
  (`==`, `assert_eq!`, `.eq`, `.ct_eq`, or a page body that `.contains` the
  cookie), which fails now that the embedded token is masked per response;
  `V13-JSONWEBTOKEN-12`, `EncodingKey::from_secret` and
  `DecodingKey::from_secret` calls, such as those of `make:cors-jwt`
  middleware, while a manifest declares `jsonwebtoken` 11 or earlier;
  `V13-ED25519-DALEK-3`, an `ed25519-dalek` 2 requirement in a workspace that
  builds Rullst's `ed25519-dalek` 3 through `rullst-iot` or
  `rullst-connect`; `V13-SQLITE-WAL-AVAILABLE`, once per SQLite project
  whose `Rullst.toml` has no `sqlite_journal_mode`, for information (WAL
  stays opt-in); and `V13-AUTH-SESSION-REGISTRY`, the session-epoch cookie
  calls of the files `cargo rullst auth`, the SaaS starter, `make:mfa` and
  `make:oauth` generated before alpha.6, while no manifest enables a session
  registry feature. The checklist's session step now lists the files to
  change and how to compare them with a project the alpha.6 CLI generates.
  `V13-CONNECT-INFALLIBLE-NEW-REMOVED` also reports the `new` of a provider
  that the package defines with `define_provider!`, in any of its files.

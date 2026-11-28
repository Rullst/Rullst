### Fixed

- The alpha.6 to rc.1 checklist of the migration guide ran
  `cargo rullst upgrade --dry-run` with the project's version (or
  `--from 13.0.0-alpha.6`), which applies only the three rules introduced in
  rc.1, so an application that followed it never ran the wider checks of
  the five alpha.6 rules that rc.1 widened (`V13-CSRF-TOKEN-COMPARED`,
  `V13-CSRF-COOKIE-IN-PAGE`, `V13-CSRF-STATIC-PREFIXES`,
  `V13-AUTH-SESSION-REGISTRY`, `V13-NON-EXHAUSTIVE-LITERAL`). Step 2 now
  also runs `cargo rullst upgrade --dry-run --from 13.0.0-alpha.5`, names
  those rules and the two that rc.1 narrowed (`V13-LOCALE-JIFF-DATES`,
  `V13-CAPITAL-INVOICE-JIFF-DATE`), and says how to mark findings the
  alpha.6 upgrade already handled with `rullst-upgrade: reviewed <CODE>`;
  the rc.1 to rc.2 checklist tells an application upgraded without it to
  run it. The warning that `cargo rullst upgrade` prints for a project
  already on the target version now ends by saying that these rules need
  `--from 13.0.0-alpha.5` or earlier. Both checklists' verification steps
  now say to run `cargo rullst generate:ai-context` and its `--check`,
  because `cargo rullst upgrade` leaves `.llms.txt` and
  `.rullst/context-map.json` at the previous versions, and step 4 says that
  an unset `MAIL_HOST` still means `127.0.0.1` with no configuration error,
  while an empty one is refused in staging and production
  ([#896](https://github.com/Rullst/Rullst/issues/896)).

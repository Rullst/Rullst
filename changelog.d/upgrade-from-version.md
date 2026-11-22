### Fixed

- `cargo rullst upgrade --dry-run` on a project whose manifests already pin
  the target version (for example right after switching to
  `=13.0.0-alpha.6`) skipped the rules introduced in that version with the
  earlier ones and printed `0 must-change, 0 review` with only a discreet
  `earlier rules skipped` line, which read as if those rules had passed. The
  plan now prints a `WARNING:` line saying that the target's own rules did
  not run, and the new `--from <VERSION>` option applies the rules introduced
  after that exact version instead of after the project's, such as
  `--from 13.0.0-alpha.5` for an alpha.6 project. `--from` cannot be newer
  than the project's version or be combined with `--all-rules`, and is
  refused when the project's version cannot be read, because every rule
  applies then; the plan
  names both versions and links the checklist for the `--from` version. The
  JSON plan's `rule_selection` gains `from_version` and `target_rules_skipped`
  (#823).

# Local deployment configuration diagnostic

The v13 `deploy:doctor` candidate inspects a bounded local configuration snapshot
without starting the application, invoking other programs, contacting a provider
or changing files. Its focused CLI tests include a real generated SaaS profile.
Its source merged in [PR #225](https://github.com/Rullst/Rullst/pull/225) after
hosted checks. The initially skipped archive gate subsequently passed in
[exact-commit validation](https://github.com/Rullst/Rullst/actions/runs/35564762822).
That retrospective repair and the remaining final release campaign are recorded
in the [delivery plan](v13-delivery-plan.md).

```bash
# Explicit file snapshot, independent of this terminal's environment:
cargo rullst deploy:doctor --env-file .env.production --json

# Inspect a selected TOML and the three allowlisted process variables:
cargo rullst deploy:doctor --config Rullst.production.toml --process-env

# Staging is an explicit target, not a production-mode alias:
cargo rullst deploy:doctor --env-file .env.staging --target staging
```

Run the command from the intended application's directory. Without `--config`,
it reads only `./Rullst.toml`, or uses Core defaults when that file is absent.
An explicitly selected file must exist. Relative file paths resolve from the
current directory; absolute paths are allowed without printing their names.
This is useful for database-free starters that have no TOML file.

Choose exactly one environment source:

- `--env-file` reads the explicitly selected literal dotenv file. The shell's
  environment is not merged with it, and no parent/default `.env` is searched.
- `--process-env` reads only `RULLST_ENV`, `APP_ENV` and `APP_KEY`. It never reads
  dotenv files or provider credentials from the process environment.
- With neither flag, environment/key inspection is explicitly incomplete and
  exits unsuccessfully. Ambient settings cannot silently make the report pass.

Resolution within that snapshot uses Core's `RULLST_ENV`, legacy `APP_ENV`, then
TOML `[app].env` precedence. Conflicting legacy variables receive a review note.
The requested target defaults to `production`; `staging` must be selected
explicitly. Invalid or blank selected values do not fall through to a lower
priority value. The running application may load or override configuration
differently: the report does not observe that process.

## Checks and output

Core's configuration validator checks browser-policy syntax and exact CORS
origins/signed-webhook paths. A zero TOML port is reported. Custom CSP, explicit
browser-policy exceptions and CSRF webhook exemptions are flagged for application
review; a syntactically valid CSP may still be weak. Unknown Core configuration
fields receive a typo/application-specific-setting review note.

The key check catches missing/short keys, known example/mock prefixes, control
characters and single-symbol keys. It intentionally does not duplicate Auth's
complete validation or infer cryptographic randomness. Supply an explicit
`APP_KEY` in the chosen environment source; legacy TOML key fallback is outside
this diagnostic's supported profile. Actual Auth validation, secure random
generation, secret custody and rotation remain necessary.

JSON uses `rullst.deployment-diagnostic.v1`, fixed check codes, `PASS`, `FAIL`,
`REVIEW` or `NOT_INSPECTED`, and fixed remediation guidance. It always includes
`deployment_verified: false` and lists controls that were not inspected.
`inspection_complete` concerns only the selected local profile. Exit zero means
the selected profile was inspected without local failures; review notes remain
visible. It is not approval to deploy. A nonzero exit accompanies incomplete
selection, invalid inputs, detected errors or output failures.

The report excludes input paths, values, database URLs, secrets and hashes of
secret-bearing files. Parser and filesystem error details are withheld. Text
output has the same observations and boundaries as JSON. Redirect JSON yourself
if a local report file is needed; the command does not create one automatically.

## Input boundary

Files must be regular UTF-8 files of at most 64 KiB. Links, reparse points and
special files are rejected, including linked supplied parent directories.
The current directory's OS aliases are resolved once, which supports macOS
temporary directories. Parent directories must be trusted against concurrent
replacement; this inspection is not a filesystem sandbox or atomic snapshot of
all application state. The parser rejects duplicate TOML fields.

The literal dotenv profile permits blank/comment lines and unique ASCII
`KEY=value` assignments, optionally prefixed by `export `, with simple matching
single/double quotes and whitespace-separated trailing comments. A `#` embedded
in an unquoted value remains part of the value. There are at most 512 assignments,
128 bytes per key and 8192 bytes per value. Interpolation (`$`), backslash escapes,
multiline/concatenated quotes, control characters and duplicate assignments are
rejected. Arbitrary dotenv syntax is not silently approximated. Other literal
keys are parsed only to validate the file and are omitted from the report.

This command does not verify provider credentials, mounted authorization or
middleware, webhook signatures, TLS, trusted proxies, distributed budgets/replay
state, readiness/drain, storage durability, host permissions or backup recovery.
Use the [deployment acceptance fixture](deployment-acceptance.md) and tests of
the actual application for those boundaries. Neither a local report nor that
fixture certifies the security of a cloud account or VPS.

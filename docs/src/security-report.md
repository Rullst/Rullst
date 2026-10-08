# Security report (`cargo rullst audit --report`)

`cargo rullst audit --report [md|html|json]` (new in 13.0) writes an evidence
report about one project: a set of bounded static checks mapped to
[OWASP ASVS 5.0.0](https://github.com/OWASP/ASVS/releases/tag/v5.0.0_release)
Level 1, a personal-data inventory and accessibility heuristics. It is
written for a reviewer.

The report **does not replace a manual security review or a penetration
test**. It never states that a requirement is met, never reports `PASS` and
is not a certification. A check that finds nothing reports `NO FINDINGS`: that
check ran and saw nothing it recognizes, nothing more.

```bash
cargo rullst audit --report            # SECURITY_REPORT.md in the project root
cargo rullst audit --report html       # SECURITY_REPORT.html, one self-contained file
cargo rullst audit --report json --output evidence/report.json
```

The terminal prints a short summary grouped as security checks, personal data
and accessibility, with ✓ (no findings), ! (not checked, exceptions or review
items) and ✗ (findings or error), a fix and a docs link per failing check. The
command exits with status 1 when any check reports `FINDINGS` or `ERROR`, and
0 otherwise. Flags, formats and the JSON schema are in the
[CLI reference](cli_reference.md#cargo-rullst-audit).

## Reading a result

| Status | Meaning |
| :--- | :--- |
| `NO FINDINGS` | The check ran over the inputs it names and recognized no problem. |
| `NO FINDINGS OUTSIDE EXCEPTIONS` | `cargo audit` passed only because of `--audit-ignore` exceptions; those advisories remain open. |
| `FINDINGS` | The check reported items; each has a location and a message. The exit status is 1. |
| `OBSERVED` | The personal-data inventory listed fields. It never changes the exit status. |
| `NOT CHECKED` | The check could not run (for example no `src/`, no Git work tree or no `cargo-audit`). It says why. |
| `ERROR` | The check started but could not complete (an unparsable `Rullst.toml`, an incomplete source walk, a failed `cargo audit`). The exit status is 1. |

Each check lists the ASVS 5.0.0 requirements it relates to, with their level.
Most are Level 1; a few related Level 2 requirements are shown for context (for
example the CSP header, V3.4.3). A mapping names what the check relates to; a
static scan only partially evidences any of them.

## Security checks

### Security headers and CSP

Looks for `Server::new`/`Server::new_hot` (the Core baseline sends HSTS,
`nosniff`, frame and CSP headers in staging and production) or a router
header layer (`headers_middleware`, `SecureHeadersLayer`). A custom
`[security] csp` in `Rullst.toml` is reported when it is empty, has no
`script-src`/`default-src`, or allows `'unsafe-inline'`, `'unsafe-eval'` or a
broad script source (`*`, `http:`, `https:`, `data:`). `coep = "unsafe-none"`
or an unknown value is reported. Maps to V3.4.1 (L1) and V3.4.3, V3.4.4 and
V3.4.6 (L2). Response-level tests against the deployment remain necessary; see
[which security layer to use](security-layers.md).

### CSRF

Lists write routes (`post`, `put`, `patch`, `delete`) declared on one line as
`method("/path" => handler)` or `.route("/path", method(handler))`. Without the
`Server` baseline and without `csrf_middleware` on the router, each write route
is a finding, except the exact paths in `[security] csrf_signed_webhook_paths`.
Machine endpoints (`with_machine_endpoints`) are noted in the evidence. Maps to
V3.5.1 (L1).

### Session and auth cookies

Reads string literals shaped like a `Set-Cookie` value (`name=value; Path=/`):
each needs `SameSite`, `Secure` (or a format placeholder that adds it outside
development) and, for session, auth or token cookies, `HttpOnly`. Cookie
builders with `.secure(false)`, `.http_only(false)` or `SameSite::None`, and
`[security] csrf_same_site = "None"`, are findings. Maps to V3.3.1 (L1) and
V3.3.2 and V3.3.4 (L2).

### Rate limiting

Finds write routes whose path names an authentication action (`login`,
`register`, `password`, `otp`, `session`, `token`, `auth`, ...). Each one is a
finding when the project mentions no limiter or login jail (`RateLimiter`,
`rate_limit_middleware`, `Server::rate_limit`, `RedisRateLimiter`,
`LoginGuard`). The evidence is project-wide: it does not prove each route sits
behind the limiter. Maps to V6.3.1 (L1).

### Committed secrets

Scans the files `git ls-files` lists (`target/`, symlinks, binaries and files
over 2 MiB are skipped) for private-key headers, AWS `AKIA` key IDs, Stripe
`sk_live_`, GitHub `ghp_`/`github_pat_` and Slack `xoxb-`/`xoxp-` tokens, and
`*_SECRET=`/`*_KEY=` assignments with values of 20 characters or more in a
committed `.env` file (`.env.example`-style templates excluded). A finding
shows the file, the line and a redacted preview: the first four characters and
`…`. The report never contains the value. Untracked files and Git history are
not scanned. A finding means: remove the value, rotate it at the provider and
load it from the environment or a secret manager. ASVS 5.0.0 has no Level 1
requirement for this; it relates to V13.3.1 (L2) in chapter V13 Configuration.

### Vulnerable dependencies

Runs the same `cargo audit` step as `cargo rullst audit`, with the same
`--audit-ignore` exceptions, on the project's `Cargo.lock` or, in a workspace
member, on the workspace root's lockfile. Without `cargo-audit` the check is `NOT CHECKED`;
a run that reports advisories or fails is `ERROR`. Maps to V15.2.1 (L1).

### IDOR

Reuses `cargo rullst audit --idor`: every parameterized route needs an
adjacent `// rullst-access: public|owner|role|admin — reason` classification
and the matching guard in the crate. Maps to V8.2.1 and V8.2.2 (L1).

## Personal-data inventory

Lists model fields marked with the ORM attributes: `#[privacy]` (or
`#[privacy(...)]`) in a `#[derive(PersonalData)]` model is `personal`,
`#[orm(encrypted)]` or a `SecretString` type is `encrypted`, and
`#[orm(masked)]` is `masked`. Each row says whether the column is stored
encrypted. Fields of ORM models with likely personal names that carry none of
these markers (`email`, `phone`, `cpf`, `cnpj`, `ssn`, `birth`, `address`,
`document`, `ip`) are listed as `review`. That is a **name heuristic**: it
neither proves nor rules out personal data. The inventory is `OBSERVED` and
never changes the exit status. It relates to chapter V14 Data Protection
(V14.1.1, L2).

## Accessibility

A tag scanner reads `html!` invocations in production Rust sources and HTML
templates (`.html`, `.htm`, `.tera`, `.hbs`, `.jinja`, `.j2`) under `src/` and
`templates/`:

| Check | Finding | WCAG 2.2 |
| :--- | :--- | :--- |
| Images have a text alternative | `<img>` without `alt` | 1.1.1 Non-text Content |
| Form controls have a label | `<input>` (except hidden, submit, button, reset, image), `<select>` or `<textarea>` without `<label for>`, a wrapping `<label>`, `aria-label` or `aria-labelledby` | 1.3.1, 3.3.2, 4.1.2 |
| Pages declare their language | `<html>` without `lang` | 3.1.1 Language of Page |

The scanner does not render templates or follow includes, so a label supplied
by an included partial is reported for review. Passing these checks does not
make a page accessible; test with a screen reader and keyboard.

## NOT EVALUATED

ASVS 5.0.0 has 70 Level 1 requirements. The checks relate to 7 of them. The
report lists the other 63 as `NOT EVALUATED`, by chapter (whole chapters such
as V1 Encoding and Sanitization or V11 Cryptography, and the remaining
requirements of partly mapped chapters). `NOT EVALUATED` is not a failure and
not a pass: a static source scan cannot establish those requirements, which
need design review, dynamic testing or deployment evidence.

## Using the report with an auditor

1. Run the report from a clean checkout of the commit under review, with the
   same `--audit-ignore` exceptions your CI uses, and keep the JSON next to the
   commit hash.
2. Give the auditor the HTML or Markdown file together with the code. Treat
   every `FINDINGS` row as a work item and every `NOT CHECKED` or
   `NOT EVALUATED` row as scope the auditor still has to cover.
3. Use the JSON (`rullst.cli-audit-report.v1`) to track results across runs or
   to feed a ticketing system; its keys are stable within the schema version.
4. Do not present the report as a certification or as ASVS compliance. It is
   one input to the audit, not its result.

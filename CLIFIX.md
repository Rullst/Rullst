# Rullst CLI Final UX Review

This file is the handoff contract for the AI agent and human testing the Rullst
CLI on the second computer. Read the repository-root `AGENTS.md` and
`docs/src/spec.md` before changing code. Those documents remain authoritative
when this handoff is incomplete or ambiguous.

## Objective

Polish and verify the existing Rullst v12 release-candidate CLI experience.
This is a stabilization pass, not a feature-development pass. Work only on the
CLI, its generated projects, and directly related tests or documentation.

Do not work directly on `main`. Use the existing branch:

```bash
git fetch origin
git switch --track origin/fix/cli-logo-animation-speed
```

If that branch already exists locally:

```bash
git switch fix/cli-logo-animation-speed
git pull --ff-only
```

## Current Preview

Commit `4cd72e81` changes only the opening logo frame delay from 110 ms to 55
ms. The same 24 frames, blue/green/orange gradients, and final signature are
preserved. The main logo animation is therefore approximately 1.27 seconds
instead of 2.53 seconds. The later command-interface pulse is unchanged.

During the second-computer review, the human tester requested one further
pacing reduction. The current uncommitted working tree uses a 26 ms delay while
retaining those same 24 frames, gradients, and final signature. Because the
last frame has no following sleep, the nominal opening duration is 23 x 26 ms =
598 ms (approximately 0.60 seconds). The later command-interface pulse remains
unchanged.

The focused logo test and the `cargo-rullst` Clippy check passed. The full
workspace pre-flight was intentionally stopped while still compiling so that
all CLI feedback can be grouped into one batch; it did not report a failure.

## What to Test

Record the exact command and selections for every problem. Include the
operating system, terminal, terminal size, Rust version, expected behavior,
actual behavior, and complete error output. Attach a screenshot or short video
for visual or animation problems when useful.

Verify at least:

1. `cargo rullst` opening animation, final colors, subtitle, menu readability,
   keyboard navigation, cancellation, and return to the previous menu.
2. Normal color output, `NO_COLOR=1`, a non-interactive output stream, and
   `RULLST_REDUCED_MOTION=1`.
3. New-project generation without optional persistence capabilities.
4. Database selection for SQLite, PostgreSQL, MySQL/MariaDB, and Turso. Turso
   must not appear twice in the same conceptual selection list.
5. Selecting zero, one, and multiple optional persistence capabilities. The
   prompt must clearly state that the selection is optional.
6. Every displayed blueprint: generation must finish, emitted files must be
   coherent, and the documented first build/start command must work.
7. `cargo rullst dash`, including narrow terminals, exit behavior, live status,
   reduced motion, and loss of access to the monitored application.
8. Development startup and hot reload, including Rust rebuild failures,
   template/static-file changes, recovery after a fixed compilation error, and
   process shutdown without orphaned children.
9. Error messages for invalid names, unavailable tools, unsupported choices,
   occupied ports, missing configuration, and database connection failures.

Do not put real credentials or personal data in reports, fixtures, screenshots,
or commits.

## Fix or Report Decision

The secondary AI agent may implement an issue directly when all of these are
true:

- the behavior is reproducible;
- the expected behavior is objective and consistent with `docs/src/spec.md`;
- the change stays within the CLI stabilization scope;
- a regression test can reasonably prove the correction; and
- the change does not introduce a new public API, feature, provider, blueprint,
  dependency, or architectural direction.

Report the issue without guessing when it is subjective, intermittent, cannot
be reproduced, changes product direction, affects architecture or public APIs,
or extends beyond the CLI. Visual preferences such as colors, pacing, symbols,
and wording should be confirmed by the human tester unless the requested result
is already explicit in this file.

## Implementation Rules

- Follow every invariant in `AGENTS.md`, including the production zero-panic
  policy and Conventional Commits.
- Preserve the current logo frames, gradients, subtitle, and final signature
  unless the human tester explicitly requests another change.
- Prefer small, focused fixes with regression tests over broad rewrites.
- Do not modify v13 work, capability scores, roadmap claims, or unrelated
  crates.
- Do not merge into `main`, publish crates, create a release or tag, force-push,
  delete branches, or dispatch heavy/manual GitHub workflows.
- Do not hide a failing test, weaken an assertion, reduce coverage, or describe
  an unverified behavior as implemented.

## Efficient Validation

During iteration, run the smallest relevant test first, followed by the CLI
crate gates:

```bash
cargo fmt --all
cargo test -p cargo-rullst
cargo clippy -p cargo-rullst --all-features -- -D warnings
git diff --check
```

Do not run the entire workspace after every visual adjustment. Once the whole
CLI feedback batch is complete, run the repository pre-flight once:

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --all -- --check
```

The primary integration agent will review the final diff and evidence before a
pull request is merged. Fuzzing, Miri, Mutations, Kani, sanitizers, OWASP ZAP,
and the other heavy release workflows run only after the final v12 candidate
commit is frozen.

## Second-computer stabilization report (2026-09-05)

Work is in progress on `fix/cli-logo-animation-speed`. The changes described
below are still uncommitted and have not reached `main`, a release, or a tag.
The generated `test1/` directory is human test evidence and must not be added,
changed, or removed by the integration agent.

### Reproduced and corrected

- Windows loaded-library cleanup reported `Acesso negado. (os error 5)` after a
  successful hot-reload load. Error codes 5 and 32 are now treated as expected
  mapped-DLL cleanup deferrals; unexpected removal errors remain visible.
- The blank HTML starter rendered without styling because its page depended on
  the Tailwind CDN while the generated security policy allowed same-origin
  styles. It now emits and loads `static/rullst.css` locally.
- The HTMX browser script did not carry the request CSP nonce. `render_page`
  now propagates the nonce while retaining the pinned HTMX version.
- Blank generated applications did not start the standalone debug Studio,
  contrary to `docs/src/spec.md`. Hot and static blank starters now start it in
  debug builds only.
- The blank starter had no branded favicon. HTML starters now receive the
  official Rullst PNG as `static/rullst.png`, and `render_page` references that
  same-origin file.
- A database-enabled blank starter displayed `Database unavailable.` in hot
  reload even while the main process and Studio were connected. `Server::run`
  already fails startup unless the configured relational database initializes,
  so the hot page now consumes that verified startup result and does not create
  a second pool across the dynamic-library runtime boundary. Static starters
  retain the real model query and unavailable-state diagnostic.
- Studio duplicated the database schema in a persistent left sidebar even
  though `/studio/migrations` already exposes linked schema tables. The sidebar
  and its HTMX out-of-band updates were removed; the migrations page retains
  the table-browser links and the main content uses the full width.
- Supplying a positional project name (`cargo rullst new my-app`) incorrectly
  skipped the blueprint, application type, database, optional storage, and hot
  reload prompts even without `--default`. A positional name now skips only the
  name prompt; `--default` remains the explicit non-interactive profile switch.
- The first attempted hot-database bootstrap exposed that new application
  manifests still declared Rust 2021 while Rullst v12 itself uses Rust 2024 and
  requires Rust 1.96. Human testing in `test4` first reproduced compiler error
  101 at `src/lib.rs`; after changing its edition, it exposed a fatal SQLx panic
  because a dynamically linked copy of Tokio cannot initialize another pool in
  the host runtime context. The duplicate bootstrap was removed. New hot and
  static applications now declare edition 2024 and explicitly record
  `rust-version = "1.96.0"`. Development remains pinned to Rust 1.98.1 without
  unnecessarily raising the public v12 MSRV.
- The final no-hot-reload blueprint smoke pass isolated missing presentation to
  SaaS and ERP. SaaS placed its pricing CSS and navigation style attributes
  inline without the request CSP nonce; ERP relied on the Tailwind CDN plus an
  inline style block, both denied by the generated same-origin/nonce policy.
  Both blueprints now emit `static/rullst.css`, link it from the page, avoid
  remote font/Tailwind styling dependencies, use responsive narrow-viewport
  rules and receive the same-origin Rullst favicon. ERP also propagates the CSP
  nonce to its pinned HTMX script and no longer renders order IDs as `#{}1`.
  Source/contract verification passed. After reinstalling the CLI and
  regenerating the affected applications, the human tester confirmed that the
  corrected SaaS and ERP pages render with their intended styling.

### Human evidence already confirmed

- The human tester replaced `images/cargo-rullst-dash.png` with a clearer,
  up-to-date screenshot of the current CLI dashboard.
- The human tester completed a smoke pass over every displayed blueprint with
  hot reload disabled. The only presentation failures reported in that pass
  were the unstyled SaaS pricing page and ERP dashboard; both still rendered
  their content, links and database-backed records, which narrowed the fault to
  CSS/CSP delivery rather than generation or ORM initialization.
- After the local stylesheet/CSP correction, the human tester regenerated and
  visually confirmed both SaaS and ERP. No remaining presentation failure was
  reported for either blueprint.
- A fresh generated blank app served `/static/rullst.css` with HTTP 200 and
  rendered the intended dark starter design.
- The HTMX counter POST completed with HTTP 200 and incremented correctly.
- The dashboard `s` shortcut opened the generated application's Studio.
- The `d` shortcut correctly did not invent documentation for a project that
  had no generated Scalar/OpenAPI surface. Use `cargo rullst make:scalar` when
  that surface is wanted.

### Focused verification completed

- `cargo fmt --all -- --check` passed.
- `cargo test -p rullst-core --lib` passed earlier in the batch: 224 tests.
- `cargo clippy -p rullst-core --all-features -- -D warnings` passed earlier in
  the batch.
- Before the v12 option-surface simplification below,
  `cargo test -p cargo-rullst --test scaffold_contracts` passed 9 tests,
  including the former 270-combination structural matrix. That evidence is
  historical and must not be presented as the current public v12 matrix.
- Focused favicon materialization, host-verified hot database state, Cargo
  manifest, Blank/Nexus feature scope, CSP nonce, Studio responsive layout, and
  positional-name wizard tests passed.
- All 7 dashboard rendering/action tests passed, including the regression that
  keeps `[d]` feedback visible when the System & Tasks pane is already full.
- `cargo test -p rullst-studio
  data_browser::tests::test_studio_layout_and_telemetry_handlers --lib` passed.
- `cargo clippy -p cargo-rullst --lib -- -D warnings` passed.
- `cargo clippy -p rullst-studio --lib -- -D warnings` passed.
- `git diff --check` reported no whitespace errors (only the repository's
  Windows LF-to-CRLF notices).
- After simplifying the v12 application profile, the focused `v12` unit-test
  filter passed 3 tests covering the fixed profile, removal of the two public
  selectors, and preservation/filtering of every optional storage add-on.
- The focused impossible-profile regression passed after adding explicit
  rejection of `--api` with a non-Blank blueprint.
- The updated `cargo-rullst` structural gate passed all 9 tests, including the
  18-form public v12 matrix.
- After the SaaS/ERP stylesheet correction, the structural gate passed all 10
  tests. The new contract proves both pages emit and reference same-origin CSS
  and favicon assets, contain no Google Fonts or Tailwind CDN styling, and keep
  the ERP HTMX nonce/controller boundary wired. The focused favicon
  materialization test also passed for Blank, SaaS and ERP.
- `cargo fmt -p cargo-rullst -- --check` passed for this stylesheet batch.
- `cargo clippy -p cargo-rullst --lib -- -D warnings` passed again after the
  simplification, and `cargo fmt --all -- --check` plus `git diff --check`
  passed. The local machine does not currently have the `mdbook` executable, so
  the documentation build itself was not rerun in this batch.

The first full `cargo test -p cargo-rullst` attempt completed all 200 unit tests
but reached an integration fixture whose offline build lacked the cached
`der 0.8.1` package. `cargo fetch --locked` then populated the cache. The human
tester asked to stop the repeated full package run and test the CLI manually
first, so the full package/workspace pre-flight must not be reported as passed.

After the manual blueprint pass was completed, the release pre-flight was
attempted again with `cargo test --workspace --all-features`. The first run
compiled the full workspace and ran all 209 `cargo-rullst` library tests: 208
passed and one dashboard test failed because it still expected the superseded
`not scaffolded` docs-shortcut wording. The assertion and warning-color match
were aligned with the current actionable `API docs unavailable` / `make:scalar`
message, and that focused regression then passed. A second full-workspace run
showed no further failure but remained inside the expensive
`generated_cli_profiles` scaffold-compilation test until the 20-minute command
limit expired. The full workspace pre-flight therefore remains incomplete and
must not be represented as passed; the branch is suitable for review and CI,
not yet for a final release claim.

The remaining release gates completed successfully:
`cargo clippy --workspace --all-features -- -D warnings` passed with zero
warnings, `cargo fmt --all -- --check` passed, and `git diff --check` reported
no whitespace errors (only the expected Windows LF-to-CRLF notices).

### Latest human retest and follow-up

- A freshly generated blank SQLite hot-reload application now reaches the
  running state without the duplicated Tokio/SQLx pool abort seen in `test4`.
- The Studio dashboard reports SQLite and two managed tables, and the removed
  database-schema sidebar did not return.
- The Studio shell previously forced viewport height and scrolled a nested main
  panel below the header. It now uses natural browser scrolling with a sticky,
  responsive header; dashboard, security, table search, and pagination layouts
  adapt at narrow breakpoints. This change still awaits the human browser
  refresh/restart check.
- The dashboard docs shortcut already detected a missing Scalar/OpenAPI
  scaffold, but its feedback could render below a full System & Tasks pane and
  make `[d]` appear inert. Shortcut feedback now also appears in a persistent
  highlighted footer notice, including the instruction for creating `/docs`;
  the shortcut label now says `api docs` instead of the ambiguous `docs`.
- The blank starter itself uses fluid width, viewport padding, and a clamped
  heading and is intrinsically mobile-friendly. The wider blueprint set has
  structural generation coverage but does not yet have automated browser
  viewport/screenshot coverage; representative mobile visual checks remain
  useful.
- The blank blueprint intentionally does not mount `/nexus`. Blog, Portfolio,
  ERP, SaaS, and LMS mount the authenticated Nexus CMS with registered models.
  The wizard label and completion output now say that Nexus is not included in
  Blank. Database-enabled Blank manifests also stopped compiling the unused
  `nexus` feature, avoiding overhead and the false impression that a CMS route
  exists.

### Reproduced and still open

- A freshly generated complete LMS with SQLite and the dynamic-library hot
  profile starts its host server and Studio, and Studio can inspect all 52
  migrated tables, but the application catalog returns `503 Catalog
  temporarily unavailable`. The loaded application library has its own ORM
  global and reports `Orm is not initialized`; running the generator from the
  Desktop instead of inside the framework repository did not cause this.
- A same-process experiment that passed host Tokio/SQLx Rust objects into the
  loaded library removed the original missing-context panic but failed the
  real HTTP request with an invalid-memory Windows application error. That
  experiment was fully removed and must not be restored: these Rust types do
  not form a stable FFI boundary. The durable fix must keep ORM and Tokio state
  on one side of the boundary or replace database-backed DLL swapping with a
  supervised process-restart development path.
- The new 26 ms logo pacing has a focused source-level duration assertion; its
  focused CLI test passed. Visual human confirmation is still pending.

### v12 CLI application-profile simplification

- Review of the five advertised frontend choices found that they were not five
  equivalent generated implementations. The audited path is server-rendered
  `html!` with HTMX. LiveView and Wasm Island are compatibility foundations that
  still need application wiring and browser evidence; Pico.css did not
  materialize and serve its stylesheet; Tera mainly added a dependency/marker;
  and some blueprints ignored the selected frontend entirely.
- Review of the three advertised ORM choices found the same product mismatch.
  Repository and Hybrid shared generator branches, while controller/service use
  varied between blueprints. Hybrid is better represented as application-owned
  mixing per module, not a third global architecture with implied parity.
- The v12 wizard and deterministic CLI now expose neither the frontend question
  nor the ORM-architecture question. The `--frontend` and `--orm` flags were
  removed from this prerelease surface. Database-backed projects use Active
  Record; full-stack pages use `html!` SSR with HTMX; API projects remain
  headless. Completion output states the selected fixed v12 profile instead of
  hiding it.
- The headless `--api` profile is now explicitly bounded to Blank. Product
  blueprints reject that flag instead of silently retaining their HTML routes
  while the completion output claims a headless application.
- The optional storage add-on question remains. Users may still select zero or
  more Turso/libSQL, MongoDB, DuckDB, SurrealDB and Qdrant capabilities, and the
  equivalent deterministic flags remain. The Turso add-on label now states that
  application integration remains explicit in v12; it must not be confused
  with full Turso-primary parity for every blueprint.
- This reduces the public structural matrix from 270 nominal combinations to 18
  meaningful v12 shapes. The expected benefit is fewer prompts, less conditional
  scaffold work and—more importantly—a much smaller validation/release surface.
  Runtime or compile-time speedups are workload-dependent and are not the main
  claim.
- The v13 target is two genuinely complete ORM architectures: Active Record and
  Data Mapper/Repository. Hybrid remains possible per module without becoming a
  global selector. Frontend work should be decomposed into independently tested
  rendering, interaction, styling and templating capabilities; only complete
  combinations with generated assets/routes and browser evidence should return
  to the CLI.

### Deferred v13 product decision: full Turso parity

- The human tester requested Turso/libSQL as a native primary-database choice
  for every blueprint and for complex applications such as an LMS or the future
  separately operated Rullst Academy.
- This is deliberately a v13 goal, not part of the current v12 CLI
  stabilization. The v12 contract remains the bounded Turso-primary Blank/API
  profile plus the generators that already have explicit Turso contracts.
- Do not expose Turso in the Blog, Portfolio, ERP, SaaS, or LMS primary-database
  menus merely as a UI change. Those starters remain SQLx-specific, and
  advertising Turso before their generated code is ported and verified would
  create broken projects.
- The v13 implementation should cover the required Turso ORM capabilities,
  backend-aware migrations and application transactions, Nexus persistence,
  Studio inspection, each blueprint's models/services, and both deterministic
  local and official remote libSQL/Turso test evidence. SQLx-specific user code
  cannot be assumed to become portable automatically.
- This goal should be developed on a dedicated feature branch in incremental,
  independently verified changes after the v12 CLI stabilization is complete.
  Until then, human testing of the other blueprints should use their displayed
  SQLx primary-database choices.

### Product recommendation: focused v12, differentiated v13

The current recommendation is not to compete on the number of selectable
features. Rullst's differentiator is the integrated path across CLI, ORM,
Studio, Nexus, security and domain crates, but that breadth is valuable only
when the generated application remains predictable. A framework with more
features does not by itself produce a better application; maturity, verified
compositions, documentation and the product team's execution remain decisive.

#### v12 release focus

- Treat v12 as a release-candidate/early-adopter product until its final gates
  and the deferred ORM/frontend audit pass on the exact release SHA. Do not
  describe the entire framework or every crate composition as generally
  production-ready based only on compilation or existing unit tests.
- Keep one golden generated web path: Active Record plus server-rendered
  `html!` and HTMX. Keep Blank API headless. Do not restore the five frontend
  choices or three global ORM choices merely to advertise a larger matrix.
- Optimize for a small number of end-to-end reliable outcomes: every public
  blueprint must generate, compile, migrate, start and serve its primary page;
  database-backed routes must prove initialized ORM state; Studio and Nexus
  links must open the supported route; and failure messages must tell the user
  how to recover.
- Disable dynamic-library hot swap on the public v12 generator surface for all
  profiles, not only LMS or database-backed blueprints. Remove its wizard
  question and reject or hide `--hot-reload`; most product blueprints use a
  database, and other process-global runtime/auth/cache/telemetry state could
  suffer the same executable-versus-DLL split even where ORM is absent.
- Preserve the development experience through supervised process restart:
  watch relevant files, coalesce changes, compile, stop only the owned child,
  start the replacement after successful compilation, and refresh the browser
  after readiness. A failed build must leave an actionable bounded diagnostic
  and must not start a broken replacement. This may be called auto-reload, but
  must not be presented as in-process/DLL hot swap.
- Keep the existing DLL implementation only as an explicitly internal
  experiment for later audit, not as an advertised v12 capability. Existing
  generated hot profiles can run their directly linked router with `cargo run`
  while this limitation is documented. Never pass Rust/Tokio/ORM objects across
  the DLL boundary as if they formed a stable FFI ABI.
- Complete the Active Record negative tests and trust-boundary audit already
  listed below. Vendor and serve the pinned HTMX client from the application,
  then verify CSP and offline behavior instead of relying on a remote CDN.
- Run real-browser checks for representative desktop and mobile viewports,
  keyboard navigation, visible focus, reduced motion, HTMX failures and basic
  accessibility. Responsive CSS is useful evidence but is not mobile-browser
  certification.
- Position the official frontend accurately: it is a web-first SSR/HTMX path,
  not a native Android/iOS UI toolkit. A mobile product can consume a Rullst API
  from a separate native, cross-platform or WebView client.
- Publish limitations alongside strengths: Turso-primary is bounded to its
  documented v12 profiles; richer frontend/ORM alternatives remain APIs or
  foundations; Studio/Nexus do not replace application authorization or
  deployment review; and provider mocks are not live-provider certification.
- Prefer a smaller, dependable CLI and faster release stabilization over new
  v12 option branches. Any build-time improvement from the reduced matrix must
  be measured; the defensible immediate benefit is lower validation and support
  complexity, not a guaranteed faster compiler or runtime.

#### v13 product direction

- Audit the hot-reload architecture before deciding whether true DLL swapping
  should return at all. Inventory every `OnceLock`/`LazyLock`, thread-local and
  process-global ORM, Tokio, auth, session, cache and telemetry state; define
  which side owns runtimes, pools, transactions and shutdown; verify atomic
  router replacement, cancellation/drain, compile-failure rollback, retained
  library limits, authentication tokens, and Windows DLL locks/crash cleanup.
  Run the result across every supported blueprint/database and both successful
  and failed reloads. If it cannot materially outperform a safe supervised
  restart without widening the Rust-ABI risk, keep process restart as the
  official v13 architecture instead of restoring hot swap for marketing parity.
- Preserve two complete ORM architectures only: Active Record and Data
  Mapper/Repository. Allow an application to mix them per module without
  presenting `Hybrid` as a third global architecture. Require equivalent
  generated CRUD, migrations, policies, transactions, Nexus/Studio behavior
  and backend matrices before exposing either choice.
- Deliver the full Turso/libSQL parity described in the preceding section,
  including every supported blueprint and explicit local/remote evidence.
- Replace monolithic frontend labels with independently composable and tested
  capabilities: rendering, client reactivity, interaction transport, styling,
  templating and asset strategy. Expose only combinations that materialize all
  required routes/assets and pass browser contracts.
- Keep SSR/HTMX as the simple default, while developing one genuinely reactive
  official profile for UI-heavy web applications. Its server/client boundary,
  hydration or non-hydration model, validation, failure recovery, accessibility
  and deployment story must be explicit and measured rather than inferred from
  dependency markers.
- Define a deliberate mobile strategy. The practical baseline is an
  authenticated, versioned API/client contract usable by native clients; PWA,
  WebView/Tauri-style shells and any future native UI integration should be
  named and tested as separate delivery targets instead of calling responsive
  HTML a native mobile application.
- Retain the integrated Studio/Nexus/security advantage, but make each tool
  report only verified runtime state and guide the operator to a concrete
  recovery action. Add upgrade documentation and executable end-to-end examples
  so framework breadth reduces user work rather than increasing uncertainty.
- Use established frameworks as design references without copying their claims:
  Loco demonstrates a narrower Rails-like models/controllers/jobs/mailers/auth
  production path; Leptos demonstrates typed reactive UI, SSR/hydration and
  server functions; Topcoat explores server-rendered async components and
  browser reactivity without a Wasm client build. Topcoat currently labels
  itself early-stage and experimental, so it is an architectural reference,
  not evidence that an equivalent Rullst feature is production-ready.

Primary comparison references reviewed on 2026-09-05:
[Loco documentation](https://loco.rs/docs/),
[Leptos documentation](https://book.leptos.dev/getting_started/index.html), and
[Topcoat repository/roadmap](https://github.com/tokio-rs/topcoat).

Decision guidance at the v12 checkpoint: Loco is currently the more predictable
choice for a conventional database-backed SaaS/API, Leptos is the stronger fit
when a highly reactive Rust web UI is the main requirement, and Topcoat is still
an explicitly experimental option. Rullst v12 is most compelling for teams that
value its integrated toolchain and accept early-adopter stabilization work. The
v13 objective is to retain that differentiation while removing the reliability
and evidence gap, not simply to accumulate more menu choices.

### Deferred deep audit: official v12 Active Record and web frontend

The human owner requested that no further ORM/frontend correction be attempted
on this computer. The following preliminary static-review findings are a
handoff for a deeper audit on the other computer. They are not a completed
security assessment, and the suspected ORM paths must first be reproduced with
isolated regression tests before changing production code.

- The foundations are not disposable prototypes. Active Record uses SQLx
  bindings, typed errors, transactions, identifier validation on many builder
  methods, compile-fail macro tests and backend-specific contract suites. The
  official web path escapes dynamic `html!` text and attributes by default,
  rejects mismatched tags during macro expansion and propagates the request CSP
  nonce to the HTMX script. Existing green tests are meaningful evidence, but
  they do not prove every composition or application call site.
- `select(&[&str])` appears to reject skipped/encrypted fields without calling
  the identifier validator used by `where_*`, `group_by` and `order_by`.
  `pluck_string`/`pluck_i32` likewise reach `to_pluck_sql(column)` without an
  obvious identifier-validation step. Because an explicit `select_raw` escape
  hatch already exists, the ordinary APIs should be audited as safe APIs and
  tested against malicious or malformed column names.
- `where_in(column, empty_values)` and `or_where_in` currently return the
  builder without adding a predicate. `delete_all` permits an empty predicate
  set. The combination may therefore turn an intended empty-set deletion into
  an unbounded delete. Reproduce this against isolated SQLite state and define
  the safe semantics before fixing it; `where_not_in` with an empty set has a
  different, mathematically true meaning and should not be changed blindly.
- The process-global ORM state uses `OnceLock`. That design works for one normal
  application process but is already known to split state across the Windows
  dynamic-library hot-reload boundary, causing the open LMS catalog failure.
  The deep audit must treat ordinary-process correctness and hot-reload process
  architecture as separate concerns rather than attempting Rust-object FFI.
- `rullst-orm/Cargo.toml` currently enables SQLite, PostgreSQL, MySQL and Any
  SQLx driver features together even when generated applications select one
  strict backend. Audit whether feature-gating those drivers can reduce build
  cost without breaking macro/type availability; this is primarily a compile
  size/time issue, not evidence of incorrect query results.
- `render_page` loads the pinned HTMX client from `unpkg.com` and does not
  currently ship it as a same-origin local asset. Audit offline reliability,
  CSP behavior and supply-chain policy, preferably vendoring the exact client
  if licensing/provenance checks allow it.
- `RawHtml` and the raw `String` fragment accepted by `render_page` are explicit
  trust boundaries. Audit every generated/framework call site to prove that
  untrusted data was escaped before wrapping; do not remove the escape hatch
  without accounting for intentionally composed HTML fragments.
- The current repository has source-level escaping/CSP tests but not a complete
  real-browser matrix for mobile viewports, keyboard navigation, accessibility,
  HTMX failure behavior, CSP enforcement and offline asset loading. Responsive
  CSS inspection alone is not mobile/browser certification.
- A focused Active Record SQLite/builder-validation command was started during
  this preliminary review but deliberately interrupted by the human before it
  completed. It produced neither a pass nor a failure and must not be cited as
  test evidence.

Recommended audit order on the other computer: first add non-destructive
regressions for `select`/`pluck` identifiers and empty `where_in` plus
`delete_all`; then run the relational backend and transaction/tenant/policy
matrices; then audit every `RawHtml` dataflow, vendor/test HTMX locally, and run
browser accessibility/mobile/CSP checks. Do not mark the v12 ORM or frontend as
release-approved solely because the existing suite is green.

The existing `test1` through `test4` scaffolds predate at least part of these
generator changes and are not valid evidence for the final output. After the
remaining representative human checks, run the deferred package/workspace
gates once for the final batch.

For the current human-testing pass, continue the remaining blueprint smoke
tests with hot reload answered `no`, preferably using SQLite first so each
blueprint is checked against the same bounded local baseline. Record the exact
wizard selections, command, first failing log line, expected/actual behavior
and a screenshot when it adds visual evidence. Do not repeatedly retest a
known DLL failure or attempt the deferred ORM/frontend fixes on this computer.
Finish this short discovery pass, hand off the consolidated report for the
deeper corrections, then reinstall the corrected CLI and regenerate fresh
projects for the final regression pass. Interrupt discovery early only for a
data-loss/security symptom, a native crash, or a blocker that prevents testing
all remaining profiles; those findings require correction before proceeding.

## Commit and Handoff

Inspect every staged file and use concise Conventional Commits, for example:

```bash
git status
git diff --check
git add -p
git commit -m "fix(cli): clarify optional persistence selection"
git push
```

At handoff, provide:

- the branch name and commit hashes;
- a concise list of reproduced and fixed issues;
- exact tests and commands run, with their results;
- unresolved or subjective observations;
- any behavior that could not be tested on that computer; and
- confirmation that the worktree is clean and no change reached `main`.

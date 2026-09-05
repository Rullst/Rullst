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


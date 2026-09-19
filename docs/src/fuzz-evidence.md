# Fuzz evidence reuse

The release still requires coverage of all 40 declared targets. A successful
`fuzzing.yml` run on the final `main` commit remains mandatory. In release mode,
the planner can credit an original successful campaign when its reviewed inputs
match the candidate; only targets without eligible evidence execute again.
`force_full: true` requests all 40 executions. Diagnostic mode remains a
five-minute, single-target check and cannot replace release evidence.

## What is compared

`.github/fuzz_evidence_inputs.py` reads committed Git objects, not the dirty
working tree. A SHA-256 identity includes tracked paths, modes and blob IDs,
the pinned execution environment, preflight and campaign commands, runner
labels, tool/action versions, flags and each fuzz package's complete contents.
Each of the ten separate fuzz workspaces retains its own dependency lockfile.

Shared production source, normal/build dependencies, compiler configuration,
the target inventory and unclassified files are global inputs: changing one
invalidates every target. A change confined to one isolated fuzz package
invalidates every sibling target in that package, including changes to its
lockfile. Package isolation is conservative: unusual path dependencies, build
scripts, includes and direct file/process/environment access make fuzz-package
inputs global. The existing ORM parser inclusion is explicitly reviewed and
its included production source is already a global input. Unsupported Git
layouts, missing history or malformed evidence stop verification.

There is a small explicit list of reviewed non-inputs, not a blanket exclusion
of test or documentation directories. It includes the two browser helpers,
the mail feedback integration test, this documentation, and release/admission
control code. The root lockfile is not used by the separate fuzz workspaces;
their ten locks remain inputs. Only the top-level `dev-dependencies` table of
`rullst-mail/Cargo.toml` is normalized out, because the mail dependency's
integration tests are not built by the fuzz workspaces. Every other field,
including target-specific, normal and build dependencies, remains an input.
The regular locked-resolution, all-feature CI and CodeQL checks still run on
the changed candidate. New exclusions require a reviewed policy change and
negative tests; they are not inferred from filenames.

## Which results qualify

The planner examines at most 30 recent runs and credits only original jobs:

- The same repository, `main`, `workflow_dispatch`, and `fuzzing.yml` identity.
- A source commit that is the candidate or a Git ancestor of it.
- A run created within the preceding seven days; reusing it does not renew age.
- A completed successful workflow and evidence boundary, a successful package
  compile preflight, and the unique successful target job from one exact attempt.
- A successful `Run bounded target campaign` step lasting at least 19,800 seconds.
- Equivalent package input and execution identities, independently recomputed.

A newer matching failed, timed-out, cancelled or running target prevents falling
back to an older success. Short diagnostics, absent/skipped targets, duplicate
job names, incomplete API pagination, foreign repositories/branches, and
unrelated commits receive no credit. Missing original evidence requires a new
execution; a reuse receipt is never a substitute for the original job.

## Publication and receipts

The current campaign emits a receipt with the candidate, policy/input digests,
newly selected targets and each reused target's original run, attempt, job,
source commit and completion time. The GitHub summary links to those runs.

The tag-only release admission check first requires a successful current-commit
fuzz workflow. If some target jobs were reused, it independently reads Git and
GitHub again and requires complete 40-target coverage; it does not trust the
uploaded receipt. It retains `release-fuzz-evidence.json` for that decision.
Expired or newly invalidated evidence blocks publication and requires another
campaign. The other 27 required workflows retain their existing exact-SHA rules.

This is reuse of bounded historical testing, not proof that every input is safe
or that another randomized campaign would find nothing new. It compares tracked
build/run inputs and pinned tool versions, not bit-identical hosted runner
images or external state. Corpora can evolve and hosted runner images change.
The seven-day limit bounds age; maintainers can always request a fresh complete
campaign. The reviewed policy code remains part of the release trust boundary.

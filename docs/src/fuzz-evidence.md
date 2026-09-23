# Fuzz evidence reuse

The v12 release still requires evidence for all 40 declared targets. A successful
`fuzzing.yml` run on the final release-branch commit (`v12` for maintenance)
remains mandatory. In release mode,
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

Production source without an admitted dependency profile, normal/build
dependency manifests, compiler configuration, the target inventory and
unclassified files remain global inputs: changing one invalidates every target.
A change confined to one isolated fuzz package
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

### Reviewed Auth dependency profile

The v12.1.1 profile admits selective attribution of `rullst-auth` package
contents. The planner resolves local Cargo paths, workspace inheritance,
renamed dependencies and the transitive union of normal, build, development,
optional and target-specific dependencies. It does not guess which individual
functions a target might call. Auth changes therefore reach all twelve facade
targets and the Nexus target, even when one of those targets calls another API.

Cargo metadata alone cannot prove that a program never reads another package's
files at runtime. `.github/fuzz-source-scope.json` also pins reviewed source
contexts for the eight independent fuzz packages: their entire conservative
local dependency closure, harnesses, locks, assets, paths, modes and root
manifest. A changed or unreviewed context receives the Auth inputs as well.
Passing a fresh campaign on that changed context does not approve a narrower
scope for a later Auth change. New context identities require source review.
The initial reviewed contexts have no harness path that reads Auth source as
data; SQLite builder fixtures use their explicit in-memory initialization.

Literal source inclusions add dependency edges. Local build scripts, unknown
dependency replacement, dynamic or ambiguous source inclusions, changed Cargo
configuration and unreviewed procedural macros restore global attribution.
The exact existing doctest module and parent are pinned separately; this is
not a blanket exclusion of Rust source. Other production crates retain global
attribution until another profile is reviewed. Unknown layouts cannot silently
receive selective credit.

This profile can reduce the current Auth correction from 40 new campaigns to
13, with up to 27 original campaigns reused **only if all provenance, age,
completion and input checks below also pass**. It does not transfer evidence
between repositories: a new private validation repository has no inherited
campaign history. Its preliminary results do not automatically admit the
public release branch.

The exact old/new blobs for the maintenance changelog, SST and review document
are recorded in `.github/fuzz-reviewed-maintenance-docs.json`. Unreviewed contents,
path/mode changes and deletion remain inputs. These files are not runtime
inputs of the reviewed fuzz profiles; a recognized runtime inclusion prevents
document normalization. The facade's executable fuzz-contract integration test
and coverage reporting workflow are reviewed control inputs, retained in their
own checks rather than the runtime fingerprint. Private coverage runs retain
the same threshold checks and GitHub artifacts without uploading to Codecov.

### Harness quality

Three historical facade harnesses only discarded their input; the session
harness used a repeated-byte key rejected before token parsing. Their historical
successful jobs establish execution of those old harnesses, not the intended
functional coverage. The corrected config, tenant, session and realtime targets
execute real APIs with deterministic positive/negative regression contracts.
Session fuzzing uses a valid fixture key and checks authenticated round trips
and nonce tampering; tenant tests require authenticated membership; realtime
checks bound payloads and prevent delivery to another tenant. The realtime
target does not claim to fuzz a TCP/WebSocket framing implementation.

The current scheduler, release gate and workflow lint reject trivially empty or
discard-only target bodies. This narrow structural check is not a semantic
coverage proof; the four executable contracts and the actual fresh campaigns
remain necessary. Changed harnesses never borrow their old campaign result.

### Reviewed v12.1 publication documentation

The publication review in commit `c3417135` changes 48 README, book and public
site files relative to `44902312`, without changing runtime source, build
scripts, dependency manifests, fuzz harnesses, locks or execution commands.
`.github/fuzz-reviewed-publication-docs.json` records the exact old/new Git blob
IDs by path. Only those two contents, with their original regular-file mode,
share an input identity; an unreviewed blob, removal, rename, mode change or new
document remains an input change. This is not a documentation-directory
exclusion. The review table itself is trusted policy included in the receipt's
policy digest.

The changed README files are Cargo package metadata, not runtime inclusions.
The book's Rust snippets are included only by `#[cfg(doctest)]`; fuzz binaries
do not compile or execute these doctests. The site template, release banner
and two site validators run in the separate documentation/browser checks.
The Core error-console test mentions `README.md` to reject an unsupported
source-file extension; it does not feed README contents to a fuzz harness.
The exact changed contents were reviewed against these consumers. A new
production consumer changes shared source and invalidates all targets.
Package/readme audits, browser checks and doctests still validate the changed
candidate; this exception only concerns the bounded fuzz campaign.

### Reviewed 12.1.1 registry documentation migration

The registry-readiness documentation changes twenty-four Markdown files relative
to `8025c213`: packaged README links, stable branch/version context,
release/security records, retired-branch cross-links and retained book anchors.
The exact before/after document snapshots are recorded in the review table. It changes no runtime source, manifest,
lockfile, build script, harness or execution command. All sixteen generated
archives were checked for exact README contents, version and MSRV; the book and
README link checks remain separate from fuzz evidence.

`.github/fuzz-reviewed-registry-docs.json` records the two exact commits and
old/new blob pairs. The policy explicitly permits only these twenty-four paths
and combines their frozen contents with earlier reviews. It preserves both
original maintenance evidence and the newer Auth evidence without admitting
later unreviewed Markdown edits. Path, mode, deletion, runtime inclusion and
unreviewed consumer rules are unchanged. All review tables are included in the
receipt's policy digest.

The macro profile retains two exact `rullst-orm-macros` tree identities because
that package's README link changed; its Rust source and manifest are identical.
A third tree is not accepted. The AI source-context review also records the
identity with its newly reviewed README removed from the potential-consumer
inventory. The facade and Nexus profiles pin the exact potential consumers
from the reviewed pre-Auth and post-Auth snapshots; both still consume Auth
through their dependency graph, so the thirteen changed Auth targets remain
ineligible for pre-Auth evidence. These are bounded trust-policy updates, not
automatic inferences that every README is safe to ignore. Negative regression tests cover
unreviewed document contents, malformed records, new paths, future macro source
and metadata changes, and unknown runtime document consumers.

Equivalent inputs only make evidence eligible. Full-duration original campaigns,
provenance, the seven-day age limit and the final release evidence boundary are
still required. A documentation-only change does not itself claim 40/40 coverage.

## Which results qualify

The planner examines at most 30 recent runs and credits only original jobs:

- The same repository, versioned release branch, `workflow_dispatch`, and
  `fuzzing.yml` identity.
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

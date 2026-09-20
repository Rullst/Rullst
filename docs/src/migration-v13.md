# Preparing an application for v13

**The current source is `13.0.0-alpha.1`, not a published stable v13 release.**
Stable v12.1.0 remains on `main`. The development packages use the same v13
version and internal requirements, while `rullst-privacy` retains `publish =
false` until its separate release admission. Do not request v13 artifacts from
the stable updater before those artifacts have actually been published.

This inventory covers the development source through the native age and
optional-consent consumers, plan-gated billing reports and the Android artifact-verification candidate. Revisit it as the remaining
[delivery priorities](v13-delivery-plan.md) land. It does not promise that every
existing application, provider account or deployment works without review.

## Changes from the published 12.1.0 source

| Area | Application impact |
| :--- | :--- |
| Existing runtime APIs | No required replacement of an existing application API was identified in this increment. Source preparation changes supported dependency requirements; it invents no Rust rewrites. |
| Security headers | Core and both Security header layers preserve an endpoint's exact `Referrer-Policy: no-referrer`, including when it occurs among duplicate values. Other endpoint values still yield to the configured baseline. Review handlers that intentionally set this more restrictive policy. |
| Android release command | `omni android --release` now requires the expected public DER certificate and trusted SDK `apksigner.jar` path, and rejects missing, unchanged, ambiguous or incorrectly signed output. Configure `RULLST_ANDROID_SIGNING_CERTIFICATE` / `RULLST_ANDROID_APKSIGNER_JAR` or the corresponding CLI options. See the [signing guide](tutorials/49-omni-android-signing.md). The old Rust helper signature remains; its environment contract is stricter. |
| Age assurance | Optional `rullst-privacy` APIs and `make:age-gate` add an authenticated native declaration journey. Existing apps acquire no age policy, verified age, guardian relationship, replay storage or camera capture by upgrading a dependency. |
| Optional consent and export | `make:privacy` adds explicit versioned choices, effective withdrawal of the demonstrated optional greeting and an authenticated own-account name/email export. It requires reviewed application installation and explicit consent-store initialization. It is not a complete export of all application data or automatic worldwide legal compliance. |
| Project context | New projects receive application instructions and a bounded metadata map. `generate:ai-context` replaces recognized legacy source-concatenation output, preserves existing `AGENTS.md`, and adds a non-writing `--check`. See the [context guide](project-context.md) for size/path/configuration limits. |
| CLI migration catalog | `rullst-upgrade-rules-v2` recognizes source majors 5, 6, 11, 12 and 13. The installed CLI must belong to the exact target major. Preparations from the earlier catalog must be prepared and verified again. Downgrades still fail. |
| SaaS plan gates | The generated `/reports/billing` route uses authenticated identity, fresh revision-fenced Stripe reconciliation and an explicit `BILLING_REPORT_PLAN_IDS` allowlist. Production requires live state; offline fixtures cannot grant access. The additive `rullst-capital::entitlements` API supports other trusted adapters. Existing applications must review/install the generated module and route; no new billing schema is needed. |
| Application templates | SaaS and `make:billing` add the report module; privacy generators remain explicit additions. Updating a dependency does not replace generated controllers or application customizations. |

The [12.0→12.1 guide](migration-v12-1.md) still applies to applications that have
not adopted the 12.1 account, payment and deployment changes. Skipping directly
to 13 does not perform those application/database migrations.

## Use the target-major CLI

A v12 CLI cannot learn v13 migration rules just by downloading another binary.
After an admitted stable v13 exists, inspect its exact release with the v12
updater's explicit `--allow-major` option, review/install its authenticated CLI,
then start a **new invocation of that v13 CLI** for the project migration.
Pre-release discovery also requires an exact `--to` and `--prerelease`.

For a source evaluation today, build the CLI from the reviewed v13 checkout:

```bash
cargo build --locked -p cargo-rullst
./target/debug/rullst --version
```

The output must identify `13.0.0-alpha.1`. Use that explicit executable path
instead of assuming a `rullst` already on `PATH` is the same binary. This is a
local source build, not authenticated published distribution evidence. Generated
pre-release projects may refer to matching local framework packages; keep that
checkout available and review those paths. The final package rehearsal must
independently prove registry-based consumers.

## Prepare, verify, review and apply

Keep a restorable application and database backup. Quiesce writers during file
application/recovery and review the application's ignored files, toolchain and
Cargo configuration. The Git working directory's tracked and non-ignored files
are copied into private preparation storage; the root `Cargo.lock` is included
even if ignored. Typical ignored secret files and build outputs are excluded.

Using the selected v13 executable, the structured sequence is:

```text
rullst update project prepare --project /path/to/app --to 13.0.0-alpha.1 --json
rullst update project verify --prepared PREPARED_DIRECTORY --dry-run --json
rullst update project verify --prepared PREPARED_DIRECTORY --allow-project-code --json
rullst update project review --verified VERIFIED_DIRECTORY --json
rullst update project apply --verified VERIFIED_DIRECTORY --approved-review REVIEW_SHA256 --json
```

Use the directories returned by the preceding commands. Inspect the complete
review before supplying its digest. Verification executes trusted build scripts,
procedural macros and tests, and is offline by default; the private copy is not
a sandbox. Supply `--allow-network` only after reviewing that separate need.
Select the application's actual feature policy (`--features`, `--all-features`
or `--no-default-features`) instead of assuming default features cover production.

Preparation accepts supported versioned dependency declarations and preserves
their TOML structure. Unversioned path/git dependencies, ambiguous ranges,
unsupported source majors, stale inputs and unresolved source findings require
manual review. Verification requires every managed framework package in the
candidate lockfile to resolve to the selected exact target. It runs locked
workspace checks and tests before review can authorize manifest/lockfile edits.

After application, source-file recovery uses the same reviewed operation:

```text
rullst update project recover --verified VERIFIED_DIRECTORY --approved-review REVIEW_SHA256 --json
```

Recovery rejects later conflicting edits. It restores the recorded manifest and
lockfile states; it cannot undo database writes, network calls, provider effects
or other external effects of application tests. It does not deploy either state.

## Privacy remains an application decision

The preview [privacy guide](privacy-age-assurance-roadmap.md) documents the explicit local source
selection and authenticated SaaS/LMS installation contracts. Do not enable an
age restriction merely because the framework offers one. Select a server-owned
policy for the actual action and accept declarations only where that policy
permits their assurance level. Production rejects unsupported stronger methods
and process-local replay storage.

Consent-store files, replay databases, encryption/signing keys, subject/tenant
binding and retention/restore procedures need their own deployment plan.
Shared-local SQLite consent is not a multi-host consensus store. The profile
export remains available independently of optional-consent configuration, but it
only exports the authenticated account fields supported by its adapter.

## Evidence and remaining acceptance

`update_project_cli` exercises distinct 12.1.0 and 13 packages through real Cargo
resolution, prepare/verify/review/apply/recover, changed inputs and wrong approval
digests. Those intentionally tiny packages prove the updater protocol, not the
framework's runtime API compatibility. The separate generated SaaS/LMS suites
compile actual framework source and execute application/authentication tests;
the age/privacy suites also exercise both opt-in installation orders.

Final release admission still requires the combined exact-commit workspace and
feature/consumer matrix, strict Clippy and formatting, package/native artifact
rehearsals, the required security/fuzz workflows, and this inventory updated for
all admitted changes. Application sandbox/provider journeys and deployment smoke
tests remain distinct from framework CI. See the [SST](spec.md#-12-assisted-framework-upgrade-contract)
and [delivery plan](v13-delivery-plan.md).

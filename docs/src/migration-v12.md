# Migrating an application to Rullst v12

Rullst v12 is a coordinated release train of 15 packages. Upgrade all direct
`rullst-*` dependencies together; mixing v12 facade/runtime crates with older
domain crates is outside the supported compatibility contract.

Choose the guide matching the application's source baseline:

- [v5 to v12](migration-v5-to-v12.md)
- [v6 to v12](migration-v6-to-v12.md)
- [v11-era dependencies to v12](migration-v11-to-v12.md)

Only v5 has a repository release tag among those three baselines. The repository
contains a v6 source snapshot, while “v11” principally identifies ecosystem
dependencies such as `rullst-connect` rather than a tagged v11 umbrella release.
The guides state these evidence limits instead of inventing release history.

## Safe upgrade procedure

1. Commit or stash the application and record its current `Cargo.lock`.
2. Back up every database and prove that the backup can be restored.
3. Run the old application's tests and save any known failures.
4. Install the exact v12 CLI version only after that RC or stable version is
   published. Do not use an unversioned install in a reproducible migration.
5. Run `cargo rullst upgrade --dry-run` from the application root and resolve
   every `BLOCKER`; use `--dry-run --json` when CI or other tooling consumes the
   versioned plan.
6. Run `cargo rullst upgrade` to execute the backed-up transaction.
7. Review `Cargo.toml`, `Cargo.lock`, every compiler-provided edit, and the
   Markdown/JSON reports under `target/rullst-upgrades/`.
8. Apply the baseline-specific manual changes below.
9. Run migrations against a disposable copy of production-shaped data.
10. Execute the application's tests, authorization negatives, and deployment
   smoke tests before merging.

The v12 upgrade command has deliberately bounded behavior:

- it discovers exact Cargo workspace members and updates standard, inline,
  workspace, target-specific and renamed versioned Rullst dependencies while
  preserving TOML comments and relative order;
- it leaves unversioned path/git dependencies untouched and reports them;
- it never rewrites valid Axum, SQLx, or Tokio imports;
- it selects source checks from a versioned migration-rule catalog and can emit
  the versioned `rullst.upgrade-plan.v1` JSON envelope;
- it snapshots manifests, the root lockfile and Rust sources before applying
  compiler-provided `cargo fix`, then runs `cargo check` for the workspace's
  selected features;
- it restores the snapshot when a gate fails unless `--keep-on-failure` was
  explicitly selected; an interrupted run can be recovered with
  `cargo rullst upgrade --restore <backup-directory>`;
- it returns failure when any gate fails and never reports “100% stable” or
  production readiness.

It does not install a new CLI globally, change application secrets, run database
migrations, prove runtime behavior, or replace the full test suite.

See the complete [assisted upgrade tutorial](tutorials/36-assisted-framework-upgrades.md)
for the v5 workflow, recovery examples, JSON contract and future-major policy.

## Version placeholder

The snippets in these guides use `12.0.0-rc.1`, the planned first public RC.
Use it only after publication and replace it with the exact v12 version being
evaluated. A prerelease must be requested explicitly by Cargo.

## Mandatory v12 review

Every baseline must review these contracts:

- [Cargo feature defaults and aliases](feature-matrix.md);
- [compatibility, MSRV, and support policy](compatibility-policy.md);
- explicit Nexus access policy and debug-only loopback convenience;
- debug-only, loopback-bound Studio deployment;
- ownership checks on parameterized data routes;
- exact CSRF webhook exemptions, WAF/body limits, and trusted proxy identity;
- deterministic offline provider credentials versus live provider validation;
- the [AI provider capability matrix](ai-provider-capabilities.md).

Do not deploy v12 solely because `cargo check` succeeds. Compilation does not
validate data conversion, authorization, provider credentials, reverse-proxy
trust, rollback, or restore behavior.

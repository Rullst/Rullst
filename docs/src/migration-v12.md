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
5. Run `cargo rullst upgrade` from the application root.
6. Review `Cargo.toml`, `Cargo.lock`, and every compiler-provided edit.
7. Apply the baseline-specific manual changes below.
8. Run migrations against a disposable copy of production-shaped data.
9. Execute the application's tests, authorization negatives, and deployment
   smoke tests before merging.

The v12 upgrade command has deliberately bounded behavior:

- it updates standard versioned dependency entries for the Rullst release-train
  packages, including inline tables;
- it leaves path-only and renamed dependencies untouched for manual review;
- it never rewrites valid Axum, SQLx, or Tokio imports;
- it runs `cargo update`, compiler-provided `cargo fix`, and `cargo check`;
- it returns failure when any gate fails and never reports “100% stable”.

It does not install a new CLI globally, change application secrets, run database
migrations, prove runtime behavior, or replace the full test suite.

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

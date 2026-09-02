# rullst-orm-macros

`rullst-orm-macros` implements the derives re-exported by `rullst-orm`.
Applications should normally depend on `rullst-orm`, not this proc-macro crate
directly, because generated code calls the matching runtime API.

## Derive and attribute contracts

| Macro | Bounded contract |
| :--- | :--- |
| `#[derive(Orm)]` | Generates the SQLx Active Record/query surface for named-field structs, or the explicitly selected bounded Turso profile with `#[orm(backend = "turso")]`. |
| `#[derive(TursoModel)]` | Generates only the typed Turso/libSQL model contract and rejects SQLx-only relations, soft deletes, hooks, policies, tenants, audit, and search behavior. |
| `#[rullst_orm::test]` | Runs an async test inside the task-scoped ORM transaction and rolls it back. Code that opens a separate connection is outside that sandbox. |
| `#[derive(PersonalData)]` | Declares application-selected personal-data fields; it is metadata, not automatic privacy compliance. |
| `#[derive(Enum)]` | Generates the portable string enum contract; native PostgreSQL/MySQL enum DDL remains separate. |
| `#[derive(Nexus)]` | Generates bounded model metadata consumed by the authenticated Nexus runtime. `#[orm(tenant = "organization_id")]` or the equivalent `#[nexus(...)]` opts a text field into Nexus-wide trusted-context scoping and makes it hidden/read-only. |

## Compile-time safety boundaries

The parser rejects unsupported model shapes, unknown backends, missing or
un-bindable tenant columns, invalid encrypted field types, unsafe audit fields,
malformed polymorphic relations, and SQLx-only behavior on Turso-primary
models. Generated query values remain parameterized by the runtime; raw SQL
escape hatches and author-supplied SQL fragments are not made safe by a derive.

Randomized encrypted fields cannot be used as ordinary generated filter/order
columns. Tenant scope and model policies are generated only when explicitly
declared; the macro does not authenticate a principal or authorize `unscoped`
access. Post-commit callbacks are process-local unless the application composes
the transactional outbox.

## Verification

The unit suite inspects generated SQL/bind ordering and zero-panic production
tokens. `trybuild` cases exercise the actual parser diagnostics rather than an
unresolved import:

```console
cargo test -p rullst-orm-macros --all-features
```

Backend runtime behavior is verified in `rullst-orm` against the corresponding
SQLite, PostgreSQL, MySQL/MariaDB, Turso, Redis, vector, and search matrices.
This crate alone does not prove those external protocols.

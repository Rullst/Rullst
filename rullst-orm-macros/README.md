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
| `#[derive(Enum)]` | Generates a closed bounded label contract shared by string parsing/display, Serde, `RullstValue` and SQLx codecs. `#[rullst_enum(type_name = "...", rename_all = "snake_case")]` and per-variant `rename` are validated at compile time; schema DDL is owned by `Blueprint::native_enum`. |
| `#[derive(Nexus)]` | Generates bounded model metadata consumed by the authenticated Nexus runtime. `#[orm(tenant = "organization_id")]` or the equivalent `#[nexus(...)]` opts a text field into Nexus-wide trusted-context scoping and makes it hidden/read-only. |

## Compile-time safety boundaries

The `Orm` parser is fail-closed. It uses structured `syn` nested-meta parsing
and rejects unknown or duplicate model/field options instead of silently
ignoring them. Every model must expose a persisted `id`; explicitly configured
tenant, soft-delete, and embedding targets must name persisted fields. Table,
column, relation-key, and pivot identifiers use the 1–64 byte portable ASCII
identifier grammar, while hook, policy, scope, and model names must also be
valid Rust identifiers.

Exactly one relation declaration is accepted per relation field. Orphan
relation options are rejected, `belongs_to_many` requires a pivot table,
`cascade_soft_delete` is limited to has-one/has-many, and polymorphic metadata
is limited to morph relations. The generated many-to-many foreign/related keys
default to the owner and related model names when omitted.

The derive recognizes `#[sqlx(skip)]`, `#[sqlx(default)]`, `#[sqlx(json)]`, and
`#[sqlx(json(nullable))]`. SQLx mappings such as `rename`, `try_from`, and
`flatten` fail compilation because the generated persistence SQL cannot honor
them safely. The parser also rejects unsupported model shapes, unknown
backends, missing or unbindable tenant columns, invalid encrypted field types,
unsafe audit fields, malformed polymorphic relations, and SQLx-only behavior
on Turso-primary models.

Generated query values remain parameterized by the runtime; raw SQL escape
hatches are caller-owned. Soft-delete sentinel expressions are compile-time
literals capped at 128 bytes and reject separators, NUL, and SQL comments, but
they remain author-supplied SQL fragments rather than parameterized data.
Database enums accept 1–64 unit variants with unique labels of at most 63 bytes
from the portable ASCII allowlist. PostgreSQL native enums require the
`strict-postgres` runtime profile; SQLx Any cannot decode its custom types.

Randomized encrypted fields cannot be used as ordinary generated filter/order
columns. Tenant scope and model policies are generated only when explicitly
declared; the macro does not authenticate a principal or authorize `unscoped`
access. Post-commit callbacks are process-local unless the application composes
the transactional outbox.

## Verification

The unit suite inspects generated SQL/bind ordering and zero-panic production
tokens. Twenty-four `trybuild` compile-fail cases exercise the actual parser
diagnostics, including duplicate/unknown options and cross-field invariants,
rather than an unresolved import:

```console
cargo test -p rullst-orm-macros --all-features
```

Backend runtime behavior is verified in `rullst-orm` against the corresponding
SQLite, PostgreSQL, MySQL/MariaDB, Turso, Redis, vector, and search matrices.
This crate alone does not prove those external protocols.

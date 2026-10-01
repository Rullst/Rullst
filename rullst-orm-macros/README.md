# rullst-orm-macros

`rullst-orm-macros` implements the derives re-exported by `rullst-orm`.
Applications should normally depend on `rullst-orm`, not this proc-macro crate
directly, because generated code calls the matching runtime API.

## Derive and attribute contracts

| Macro | Bounded contract |
| :--- | :--- |
| `#[derive(Orm)]` | Generates the SQLx Active Record/query surface for named-field structs, or the explicitly selected bounded Turso profile with `#[orm(backend = "turso")]`. |
| `#[derive(TursoModel)]` | Generates only the typed Turso/libSQL model contract and rejects SQLx-only relations, soft deletes, hooks, policies, tenants, audit, and search behavior. |
| `#[rullst_orm::test]` | Runs an async test inside the task-scoped ORM transaction and rolls it back. The declared return type is kept, so a test returning `Result` can use `?` and fails on `Err` after the rollback. Post-commit effects of sandboxed writes (`committed` observers, cache, Redis, Scout) are discarded like on any rollback. Code that opens a separate connection is outside that sandbox. |
| `#[derive(PersonalData)]` | Declares application-selected personal-data fields (`#[privacy]`, listed by `ComplianceModel::personal_fields()` in v13) and redacts them from `Debug`; it is metadata, not automatic privacy compliance. Its `PrivacyReport` names the ORM table (`#[orm(table)]` or the `<struct>s` default) and lists as encrypted only `#[orm(encrypted)]` and `SecretString` columns. |
| `#[derive(Enum)]` | Generates a closed bounded label contract shared by string parsing/display, Serde, `RullstValue` and SQLx codecs. `#[rullst_enum(type_name = "...", rename_all = "snake_case")]` and per-variant `rename` are validated at compile time; schema DDL is owned by `Blueprint::native_enum`. |
| `#[derive(Nexus)]` | Generates bounded model metadata consumed by the authenticated Nexus runtime. `#[orm(tenant = "organization_id")]` or the equivalent `#[nexus(...)]` opts a text field into Nexus-wide trusted-context scoping and makes it hidden/read-only. ORM relation fields and `#[orm(skip)]`/`#[sqlx(skip)]` fields are left out of the metadata; `#[orm(encrypted)]`, `SecretString` and `#[orm(hidden)]` fields become hidden, read-only `Password` fields (only `#[nexus(kind = "password")]` exposes an `#[orm(hidden)]` field, write-only), and `#[orm(masked)]` fields default to `Password`. Other shared `#[orm(...)]` options are skipped. |

## Compile-time safety boundaries

The `Orm` parser is fail-closed. It uses structured `syn` nested-meta parsing
and rejects unknown or duplicate model/field options instead of silently
ignoring them. Every model must expose a persisted `id`; explicitly configured
tenant, soft-delete, and embedding targets must name persisted fields. Table,
column, relation-key, and pivot identifiers use the 1–64 byte portable ASCII
identifier grammar, while hook, policy, scope, and model names must also be
valid Rust identifiers. Persisted Rust field names follow that same portable
SQL grammar: raw identifiers, non-ASCII names, and names longer than 64 bytes
are rejected before SQL generation. A field that cannot form its generated
Rust column-enum variant receives a compile error instead of a macro panic.
Generated SQL emits these identifiers unquoted and the derive does not check
reserved words, so a table or column named like a keyword of the target
database (`order`, `desc`, `user` on PostgreSQL, `groups` on MySQL 8, a
default `groups` table for a `Group` struct) compiles but fails at runtime;
rename it or set `#[orm(table = "...")]`.

Exactly one relation declaration is accepted per relation field. Orphan
relation options are rejected, `local_key` is rejected on `belongs_to`/`morph_to`
and `related_key` on has-one/has-many/morph-one/morph-many relations (which
would ignore them), `belongs_to_many` requires a pivot table,
`cascade_soft_delete` is limited to has-one/has-many whose related model also
uses soft deletes (otherwise the generated cascade fails to compile at the
relation field rather than hard-deleting the children), and polymorphic metadata
is limited to morph relations. The generated many-to-many foreign/related keys
default to the owner and related model names when omitted; an omitted
`foreign_key` defaults to `<related model>_id` on `belongs_to` and to
`<owner model>_id` on has-one/has-many (lowercased model names).

The derive recognizes `#[sqlx(skip)]`, `#[sqlx(default)]`, `#[sqlx(json)]`, and
`#[sqlx(json(nullable))]`. `#[orm(skip)]` only removes a field from generated
SQL; the application's `FromRow` still reads it, so a field without a table
column also needs `#[sqlx(skip)]` (the two may be combined) or
`#[sqlx(default)]`. A `#[sqlx(json)]` field is decoded through SQLx `Json`
and generated writes bind it as `Json(value)` (a `json(nullable)` `None` as
`NULL`), so a Serde-only type works on a JSON column; SQLx provides `Json` only
for the strict driver features. SQLx mappings such as `rename`, `try_from`, and
`flatten` fail compilation because the generated persistence SQL cannot honor
them safely. The parser also rejects unsupported model shapes, unknown
backends, missing or unbindable tenant columns, invalid encrypted field types,
unsafe audit fields, malformed polymorphic relations, and SQLx-only behavior
on Turso-primary models.

Generated query values remain parameterized by the runtime; raw SQL escape
hatches are caller-owned. Soft-delete sentinel expressions are compile-time
literals capped at 128 bytes and reject separators, NUL, and SQL comments, but
they remain author-supplied SQL fragments rather than parameterized data.
Comment rejection covers `--`, block-comment delimiters, and MySQL's `#`.
Database enums accept 1–64 unit variants with unique labels of at most 63 bytes
from the portable ASCII allowlist. PostgreSQL native enums require the
`strict-postgres` runtime profile; SQLx Any cannot decode its custom types.
Builder comparisons on a field whose type implements `DatabaseEnum` bind
`CAST(? AS "<type_name>")` on PostgreSQL; the derive detects such fields at
compile time through the field type, not its name.

Optional generated APIs follow the runtime's features, not the application's:
`rullst-orm` opts this crate into `runtime-feature-gates` and forwards its
`redis` and `ai` features, so the Redis cache/hash/event code and
`save_with_embedding` are emitted or omitted at expansion time. Without that
opt-in (an older runtime) the output keeps the legacy
`#[cfg(feature = "redis")]`/`#[cfg(feature = "ai")]` attributes, which the
invoking crate evaluates.

Randomized encrypted fields and `SecretString` columns cannot be used as
ordinary generated filter/order/group columns or plucked (`SecretString`
columns can still be selected, because their codec decrypts them). Tenant scope and model policies are generated only when explicitly
declared; the macro does not authenticate a principal or authorize `unscoped`
access. Post-commit callbacks are process-local unless the application composes
the transactional outbox.

Generated mandatory scopes form separate `AND` groups around user filters;
empty positive `IN` predicates remain false. Scalar projections and nested
queries preserve identifier validation and the managed transaction context.
Policy models reject bulk deletion rather than skipping per-row checks. See
the [ORM query and transaction boundaries](https://github.com/Rullst/Rullst/blob/v12.1.0/rullst-orm/README.md#query-and-transaction-boundaries)
for the explicit raw-transaction and streaming limitations.

## Verification

The unit suite inspects generated SQL/bind ordering and zero-panic production
tokens. Twenty-six `trybuild` compile-fail cases exercise the actual parser
diagnostics, including duplicate/unknown options and cross-field invariants,
rather than an unresolved import:

```console
cargo test -p rullst-orm-macros --all-features
```

Backend runtime behavior is verified in `rullst-orm` against the corresponding
SQLite, PostgreSQL, MySQL/MariaDB, Turso, Redis, vector, and search matrices.
This crate alone does not prove those external protocols.

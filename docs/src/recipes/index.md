# Recipes: application-owned integrations

Rullst keeps an integration inside the framework when the framework adds real
value to it: security work an application should not repeat (signature
verification, constant-time checks, encryption, bounded parsing), a
deterministic offline mode for tests and local work, typed errors, and limits
on what a call can read or return. A thin wrapper over an official SDK adds
little of that, yet every one of them is code the framework has to build, test,
pin and upgrade for every 13.x release. To keep 13.x maintainable by a small
team, Rullst documents such integrations as recipes instead: application code
that uses the official SDK directly and follows the framework's conventions.

13.0.0-alpha.5 removed three adapters this way:

- **MongoDB** (`MongoDbStore`, the `orm-mongodb` feature): a thin layer over
  the official `mongodb` driver. An application that uses the driver directly
  gets its whole query and index API instead of a five-method subset.
- **DuckDB** (`DuckDbStore` and the `AnalyticsRepository` API, the
  `orm-duckdb` feature): the heaviest dependency in the workspace. Its bundled
  C++ engine took most of a cold build, it once needed a security-advisory
  exception for the `zip` release its build script used, it broke the CI
  cache, and no application was known to use it.
- **Qdrant** (`QdrantStore` and the `VectorRepository` API, the `orm-qdrant`
  feature): four REST operations (create a collection, upsert, delete, search)
  with no tenant filter. The official `qdrant-client` gives an application
  filters, payload indexes and the whole API; vectors next to their rows stay
  in PostgreSQL with `pgvector`.

The [migration guide](../migration-v13.md#upgrading-an-alpha4-application-to-alpha5)
lists what changed, and `cargo rullst upgrade` reports the removed features and
types in an application.

## What a recipe is

The code on these pages belongs to your application. Copy it, rename it and
change it; Rullst makes no stability promise for it, and a future Rullst
release will not change it under you. You choose, pin and upgrade the SDK
yourself (commit `Cargo.lock`, let Dependabot propose updates and keep
`cargo audit` in CI, as generated projects do).

Every recipe has the same shape:

- **configuration** read with `rullst::config::project_setting` (the process
  environment first, then `./.env`), never hard-coded;
- **one client** created at start-up and shared with handlers through an
  `Extension` layer;
- **a small application trait** that handlers depend on, with the SDK-backed
  implementation and an in-memory fake, so tests and local work run offline,
  in the spirit of section 3.5 of Rullst's
  [`AGENTS.md`](https://github.com/Rullst/Rullst/blob/main/AGENTS.md);
- **typed errors** that never send driver details to the client;
- **bounds** on every query: a row cap and, where the engine supports it, a
  time limit;
- **tests**: the application logic against the fake, and the SDK-backed
  implementation against a real engine when you ask for it.

The application-side code of each page (the trait, the fake, the handler and
its test) is compiled and run by Rullst's CI as a doctest. Code that calls the
SDK is not: the Rullst workspace deliberately does not depend on these SDKs,
so those blocks are checked by reading them against the SDK version each page
names.

## Recipes

- [MongoDB with the official driver](mongodb.md)
- [DuckDB analytics with the official crate](duckdb.md)
- [Qdrant vector search with the official client](qdrant.md)

# Database Support

Read this page before you choose a database. It is the one place that says
what each backend works with in Rullst 13.0.

Rullst integrates one family of databases fully: SQLite, PostgreSQL and
MySQL/MariaDB, through SQLx. `#[derive(Orm)]` models and their query builder,
`Schema` migrations, `artisan!` and the `db:*` commands, `Server::with_db`,
Nexus and Studio's database pages all use that SQLx connection pool.

Turso/libSQL can be the primary database of the Blank starter, but through a
separate adapter (`TursoOrm`, `TursoModel`, `TursoRepository`,
`TursoMigration`) that does not plug into any of those parts. Integrating it is
roadmap work, not part of 13.0.

MongoDB, DuckDB, SurrealDB, Qdrant and Redis data structures are side stores:
you add them next to the primary database and call them through their own
APIs. They never replace the primary database, and there is no transaction
across stores.

## Capability table

| Backend | Primary database | `#[derive(Orm)]` and query builder | `Schema` migrations | `artisan!` and `db:*` | `Server::with_db` | Nexus | Studio | Transactions | `cargo rullst new` | Live CI test |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| SQLite | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Every starter; the default (`--database sqlite`) | No server (in-process): `matrix_sqlite`, Studio `data_browser_mutation_sqlite`, Nexus `tenant_audit` |
| PostgreSQL | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Every starter (`--database postgres`) | `matrix_postgres`, Studio `data_browser_mutation_postgres`, Nexus `sql_matrix_postgres` |
| MySQL / MariaDB | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Yes, but DDL commits implicitly (note 4) | Every starter (`--database mysql` or `--database mariadb`) | `matrix_mysql` (MySQL 8.0), `matrix_mariadb`, Studio `data_browser_mutation_mysql` and `_mariadb`, Nexus `sql_matrix_mysql` (MySQL only) |
| Turso / libSQL | Blank starter only (note 1) | No: a separate model API (note 2) | No: SQL `TursoMigration`s (note 3) | No `artisan!`; the Blank starter answers `db:migrate`, `db:rollback` and `db:status` itself (note 3) | No | No | No (note 5) | Statement batches only (`TursoStore::transaction`) | Primary: Blank, web page or `--api` (`--database turso`). Side store: `--turso` with any starter | `matrix_turso` against `libsql-server` v0.24.32 |
| MongoDB | No | No (`MongoDbStore`) | No | No | No | No | No | No | Side store: `--mongodb` with any starter | `matrix_mongodb` |
| DuckDB | No | No (`DuckDbStore`) | No | No | No | No | No | No | Side store: `--duckdb` with any starter | None: an in-process engine, covered by unit tests |
| SurrealDB | No | No (`SurrealDbStore`) | No | No | No | No | No | No | Side store: `--surrealdb` with any starter | `matrix_surrealdb` (SurrealDB 3.2.4) |
| Qdrant | No | No (`QdrantStore`) | No | No | No | No | No | No | Side store: `--qdrant` with any starter | `matrix_qdrant` |
| Redis data | No | No (`RedisDataStore`) | No | No | No | No | No (note 6) | No: each command is atomic on its own | Side store: `--redis` with any starter (it also enables Redis queues and cache) | `matrix_redis_data` |

Notes:

1. `cargo rullst new` refuses `--database turso` with any blueprint other than
   Blank. The LMS, SaaS, Blog, Portfolio and ERP starters use SQLx models,
   `Schema` migrations and Nexus, so they need a SQLx database.
2. `#[derive(Orm)]` with `#[orm(backend = "turso")]` generates the `TursoModel`
   contract instead of SQLx code: `all`, `find`, `save`, `create`, `delete`,
   `count` and `paginate`, plus filtered reads through
   `TursoOrm::repository::<Model>().query()` (`where_eq`, `order_by`, `limit`,
   `offset`, `get`, `first`, `count`). It has no SQLx query builder, and the
   derive refuses relations, soft deletes, hooks, scopes, tenant columns,
   policies, auditing, Scout search and embeddings at compile time.
3. A Turso migration is a `TursoMigration` holding SQL statements, checksummed
   and reversible with `with_down`; the `Schema` and `Blueprint` builders never
   apply to Turso. `cargo rullst make:migration` writes one, and
   `make:migration:auto` refuses Turso projects. The Blank starter's `main.rs`
   answers `db:migrate`, `db:rollback` and `db:status`, and `db:seed` reports
   that no seeders exist; `cargo rullst db:*` runs them through `cargo run`.
4. MySQL and MariaDB commit each DDL statement implicitly, so a migration's
   schema changes do not roll back with it; data statements are transactional
   on all three SQLx backends. See
   [Tutorial 05](tutorials/05-migrations-and-seeds.md).
5. Studio still starts in a Turso Blank project, but its database, migration
   and data browser pages read only a SQLx `DATABASE_URL`, so they report that
   no database is configured.
6. Studio's cache page shows the Core `Cache` handed to it (memory or Redis),
   not `RedisDataStore` structures.

## Where the live tests run

The live tests start the database in a container (Testcontainers) in the
**Run provider matrices** step of the Rust CI workflow's Linux workspace
shard. `matrix_postgres`, `matrix_mysql` and `matrix_sqlite` also run in the
**ORM strict-postgres**, **ORM strict-mysql** and **ORM strict-sqlite** jobs.
The tests live in `rullst-orm/tests/`, `rullst-studio/tests/` and
`rullst-nexus/tests/`, and each pins its container image. A live test proves
the adapter's contract against that server version; it does not certify a
hosted service, backups, replication or failover.

## Choosing

- Start with SQLite; move to PostgreSQL, MySQL or MariaDB for a shared server.
  Every starter and every framework feature works with these.
- Choose Turso as the primary database only for a Blank starter (a web page or
  a JSON API) where you are ready to write SQL migrations and use the
  `TursoModel` API without Nexus, Studio's database pages, `Schema` or the
  SQLx query builder.
- Add a side store when the application needs documents, analytics, graph
  reads, vectors or Redis structures next to its primary database. Enable only
  the adapters you use: each one is its own Cargo feature (`orm-turso`,
  `orm-mongodb`, `orm-duckdb`, `orm-surrealdb`, `orm-qdrant`, `orm-redis` on
  `rullst`), and DuckDB's bundled C++ build makes a first build noticeably
  longer.

The APIs of Turso and the side stores are described in
[Polyglot Persistence](polyglot-persistence.md); the SQLx ORM in
[rullst-orm](crates/orm.md); migrations in
[Tutorial 05](tutorials/05-migrations-and-seeds.md); every Cargo feature in the
[feature matrix](feature-matrix.md).

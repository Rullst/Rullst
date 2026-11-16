### Added

- `rullst-orm` can set the journal mode of the application's SQLite database:
  `Orm::init_with_config(url, OrmConfig::default().with_sqlite_journal_mode(SqliteJournalMode::Wal))`
  (or `SqliteJournalMode::Delete`), and an application that `Server` starts
  sets `sqlite_journal_mode = "wal"` or `"delete"` under `[database]` in
  `Rullst.toml`, which `Server` and the Artisan commands (`db:migrate`, …)
  apply. The first connection sets the mode and checks the mode SQLite
  reports; a read-only database or a filesystem without shared memory fails
  initialization with `Error::DatabaseError`. PostgreSQL/MySQL URLs and
  in-memory SQLite ignore it. Without the setting nothing changes: `Orm::init`
  and existing applications keep the mode recorded in the database file, and
  `synchronous` stays `FULL` in both modes. `cargo rullst doctor` now names
  the WAL journal of a SQLite file (counting its `-wal` file in the size) and
  warns when a WAL database is on an NFS, SMB, 9P, AFS or Ceph mount;
  `deploy:doctor` accepts the new key and rejects other values.

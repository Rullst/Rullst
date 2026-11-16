### Changed

- Projects that `cargo rullst new` generates with SQLite (Blank, SaaS and API
  starters) set `sqlite_journal_mode = "wal"` in `Rullst.toml`, with a comment
  on why and how to turn it off: in WAL mode readers stop locking the database
  file for every statement and stop waiting for the writer. SQLite then keeps
  `rullst.db-wal` and `rullst.db-shm` next to the database, so the database
  file alone is no longer a backup, and WAL must not be used on a network
  filesystem (NFS, SMB, Azure Files: set `"delete"` there). The `Dockerfile`
  and `compose.yaml` that `cargo rullst deploy` writes say so beside the data
  volume, and the VM guide's backup section adds the restore command
  (`sqlite3 ".restore"` with the app stopped). Existing projects are not
  changed; see the "SQLite WAL journal" row of the migration guide to opt in.

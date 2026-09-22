pub(super) const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS rullst_recurring_control (
        namespace TEXT PRIMARY KEY, binding BYTEA NOT NULL,
        last_now BIGINT NOT NULL CHECK(last_now >= 0))",
    "CREATE TABLE IF NOT EXISTS rullst_recurring_definitions (
        namespace TEXT NOT NULL, name TEXT NOT NULL, generation TEXT NOT NULL,
        content BYTEA NOT NULL, created BIGINT NOT NULL, next_due BIGINT,
        cancelled BOOLEAN NOT NULL DEFAULT FALSE,
        PRIMARY KEY(namespace,name), UNIQUE(namespace,generation))",
    "CREATE TABLE IF NOT EXISTS rullst_recurring_occurrences (
        namespace TEXT NOT NULL, id TEXT NOT NULL, schedule TEXT NOT NULL,
        generation TEXT NOT NULL, due BIGINT NOT NULL, created BIGINT NOT NULL,
        expires BIGINT NOT NULL, content BYTEA, state TEXT NOT NULL
            CHECK(state IN ('pending','leased','published','dead','cancelled')),
        available BIGINT NOT NULL, attempts BIGINT NOT NULL DEFAULT 0 CHECK(attempts >= 0),
        version BIGINT NOT NULL DEFAULT 0 CHECK(version >= 0),
        lease_hash BYTEA, lease_until BIGINT, terminal_at BIGINT,
        PRIMARY KEY(namespace,id), UNIQUE(namespace,generation,due))",
    "CREATE INDEX IF NOT EXISTS rullst_recurring_due
        ON rullst_recurring_definitions(namespace,next_due,name) WHERE NOT cancelled",
    "CREATE INDEX IF NOT EXISTS rullst_recurring_ready
        ON rullst_recurring_occurrences(namespace,state,available,id)",
];

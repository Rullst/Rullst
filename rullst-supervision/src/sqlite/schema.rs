pub(super) const SCHEMA: &[(&str, &str)] = &[
    (
        "rullst_supervision_meta",
        "CREATE TABLE rullst_supervision_meta (id INTEGER PRIMARY KEY CHECK(id = 1), version INTEGER NOT NULL CHECK(version = 1), config TEXT NOT NULL, last_now INTEGER NOT NULL CHECK(last_now >= 0), revision INTEGER NOT NULL CHECK(revision >= 0))",
    ),
    (
        "rullst_supervision_grants",
        "CREATE TABLE rullst_supervision_grants (tenant TEXT NOT NULL, subject TEXT NOT NULL, resource TEXT NOT NULL, delegate TEXT NOT NULL, action INTEGER NOT NULL CHECK(action IN (1,2)), revision INTEGER NOT NULL CHECK(revision > 0), expires_at INTEGER NOT NULL CHECK(expires_at > 0), revoked INTEGER NOT NULL CHECK(revoked IN (0,1)), operator_ref TEXT NOT NULL, evidence_ref TEXT NOT NULL, PRIMARY KEY(tenant,subject,resource,delegate,action))",
    ),
    (
        "rullst_supervision_sessions",
        "CREATE TABLE rullst_supervision_sessions (id TEXT PRIMARY KEY NOT NULL, tenant TEXT NOT NULL, subject TEXT NOT NULL, resource TEXT NOT NULL, policy TEXT NOT NULL, notice TEXT NOT NULL, state INTEGER NOT NULL CHECK(state IN (1,2,3)), revision INTEGER NOT NULL CHECK(revision > 0), started_at INTEGER NOT NULL CHECK(started_at >= 0), expires_at INTEGER NOT NULL CHECK(expires_at > started_at), retain_until INTEGER NOT NULL CHECK(retain_until > expires_at), sequence INTEGER NOT NULL CHECK(sequence >= 0), last_event_at INTEGER, event_count INTEGER NOT NULL CHECK(event_count >= 0))",
    ),
    (
        "rullst_supervision_session_scope",
        "CREATE INDEX rullst_supervision_session_scope ON rullst_supervision_sessions(tenant,subject,resource)",
    ),
    (
        "rullst_supervision_events",
        "CREATE TABLE rullst_supervision_events (session_id TEXT NOT NULL REFERENCES rullst_supervision_sessions(id) ON DELETE CASCADE, sequence INTEGER NOT NULL CHECK(sequence > 0), kind INTEGER NOT NULL CHECK(kind IN (1,2)), received_at INTEGER NOT NULL CHECK(received_at >= 0), expires_at INTEGER NOT NULL CHECK(expires_at > received_at), PRIMARY KEY(session_id,sequence))",
    ),
    (
        "rullst_supervision_event_expiry",
        "CREATE INDEX rullst_supervision_event_expiry ON rullst_supervision_events(expires_at)",
    ),
    (
        "rullst_supervision_managed",
        "CREATE TABLE rullst_supervision_managed (tenant TEXT NOT NULL, subject TEXT NOT NULL, resource TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision > 0), operator_ref TEXT NOT NULL, evidence_ref TEXT NOT NULL, policy_actor TEXT, not_before INTEGER, expires_at INTEGER, PRIMARY KEY(tenant,subject,resource), CHECK((policy_actor IS NULL AND not_before IS NULL AND expires_at IS NULL) OR (policy_actor IS NOT NULL AND not_before IS NOT NULL AND expires_at IS NOT NULL AND not_before >= 0 AND expires_at > not_before)))",
    ),
    (
        "rullst_supervision_courses",
        "CREATE TABLE rullst_supervision_courses (tenant TEXT NOT NULL, subject TEXT NOT NULL, resource TEXT NOT NULL, course TEXT NOT NULL, PRIMARY KEY(tenant,subject,resource,course), FOREIGN KEY(tenant,subject,resource) REFERENCES rullst_supervision_managed(tenant,subject,resource) ON DELETE CASCADE)",
    ),
];

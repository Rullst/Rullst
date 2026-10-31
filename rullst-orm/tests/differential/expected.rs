//! The documented differences from SQLite. Each entry names the backends,
//! the step, the exact value those backends record, and why. A backend that
//! produces any other value, or that starts matching SQLite, fails the
//! comparison, so this list stays an accurate description of the backends.

use serde_json::Value;

use crate::backend::Backend;

/// Why a backend may differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The database's own, documented semantics; the ORM passes them through.
    Provider,
    /// An ORM defect awaiting a decision. The comparison accepts the value so
    /// the defect stays visible and pinned, not hidden; the report prints
    /// `DEFECT` for it.
    #[allow(dead_code)]
    Defect,
}

pub struct Difference {
    pub backends: &'static [&'static str],
    pub key: &'static str,
    /// The JSON the listed backends record.
    pub value: &'static str,
    pub kind: Kind,
    pub reason: &'static str,
}

impl Difference {
    pub fn value(&self) -> Value {
        serde_json::from_str(self.value)
            .unwrap_or_else(|error| panic!("documented value of `{}`: {error}", self.key))
    }
}

const SERVERS: &[&str] = &["postgres", "mysql", "mariadb"];
const POSTGRES: &[&str] = &["postgres"];
const MYSQL_FAMILY: &[&str] = &["mysql", "mariadb"];

const CONSTRAINT_NAME: &str = "PostgreSQL names the violated constraint; SQLite and MySQL/MariaDB \
     report none, so `constraint` is `None` there (docs/src/crates/orm.md, Database errors).";
const CASE_INSENSITIVE: &str = "The MySQL/MariaDB default collations are case- and \
     accent-insensitive (`_ai_ci`/`_general_ci`); SQLite's BINARY and PostgreSQL's collation \
     compare exactly.";
const LINGUISTIC_ORDER: &str = "Text sorts by the database collation: SQLite's BINARY compares \
     bytes (uppercase before lowercase, accented letters last); PostgreSQL (glibc en_US.utf8) and \
     MySQL/MariaDB sort linguistically, each with its own rules for punctuation.";

pub const DIFFERENCES: &[Difference] = &[
    Difference {
        backends: POSTGRES,
        key: "errors.unique_violation",
        value: r#"{"error":"UniqueViolation","constraint":"diff_accounts_email_key"}"#,
        kind: Kind::Provider,
        reason: CONSTRAINT_NAME,
    },
    Difference {
        backends: POSTGRES,
        key: "collation.unique_exact_duplicate",
        value: r#"{"error":"UniqueViolation","constraint":"diff_people_email_key"}"#,
        kind: Kind::Provider,
        reason: CONSTRAINT_NAME,
    },
    Difference {
        backends: POSTGRES,
        key: "transactions.violation_is_returned",
        value: r#"{"error":"UniqueViolation","constraint":"diff_ledger_name_key"}"#,
        kind: Kind::Provider,
        reason: CONSTRAINT_NAME,
    },
    Difference {
        backends: POSTGRES,
        key: "errors.foreign_key_violation",
        value: r#"{"error":"ForeignKeyViolation","constraint":"diff_memberships_account_fkey"}"#,
        kind: Kind::Provider,
        reason: CONSTRAINT_NAME,
    },
    Difference {
        backends: POSTGRES,
        key: "errors.delete_referenced",
        value: r#"{"error":"ForeignKeyViolation","constraint":"diff_memberships_account_fkey"}"#,
        kind: Kind::Provider,
        reason: CONSTRAINT_NAME,
    },
    Difference {
        backends: SERVERS,
        key: "errors.varchar_overflow",
        value: r#"{"error":"DatabaseError"}"#,
        kind: Kind::Provider,
        reason: "SQLite does not enforce a VARCHAR(n) length (type affinity) and stores the value; \
                 PostgreSQL and MySQL/MariaDB (default strict SQL mode) reject it.",
    },
    Difference {
        backends: POSTGRES,
        key: "values.text_with_nul",
        value: r#"{"error":"DatabaseError"}"#,
        kind: Kind::Provider,
        reason: "PostgreSQL text cannot contain the NUL character (U+0000).",
    },
    Difference {
        backends: POSTGRES,
        key: "ordering.nulls_ascending",
        value: r#"["cat","dan","ann","fay","eve","ben"]"#,
        kind: Kind::Provider,
        reason: "PostgreSQL sorts NULL after every value in ascending order; SQLite and \
                 MySQL/MariaDB sort it first. The builder has no NULLS FIRST/LAST option.",
    },
    Difference {
        backends: POSTGRES,
        key: "ordering.nulls_descending",
        value: r#"["ben","eve","fay","ann","dan","cat"]"#,
        kind: Kind::Provider,
        reason: "PostgreSQL sorts NULL first in descending order; SQLite and MySQL/MariaDB last.",
    },
    Difference {
        backends: POSTGRES,
        key: "transactions.continue_after_caught_violation",
        value: r#"{"error":"DatabaseError"}"#,
        kind: Kind::Provider,
        reason: "After any failed statement PostgreSQL aborts the whole transaction and rejects \
                 further statements until it ends; SQLite and MySQL/MariaDB undo only the failed \
                 statement. A nested `Orm::transaction` (savepoint) around the statement makes \
                 the recovery portable (`continue_after_savepoint_violation`).",
    },
    Difference {
        backends: POSTGRES,
        key: "transactions.after_caught_violation",
        value: r#"[]"#,
        kind: Kind::Provider,
        reason: "The aborted PostgreSQL transaction above rolled back, so neither row exists.",
    },
    Difference {
        backends: SERVERS,
        key: "transactions.id_step_over_rolled_back_insert",
        value: "2",
        kind: Kind::Provider,
        reason: "PostgreSQL sequences and InnoDB AUTO_INCREMENT counters are not transactional: a \
                 rolled-back insert leaves a gap. SQLite's AUTOINCREMENT counter (sqlite_sequence) \
                 rolls back with the transaction, so the id is reused.",
    },
    Difference {
        backends: SERVERS,
        key: "temporal.round_trip.nanos",
        value: r#""2026-10-15T09:30:00.987654000Z""#,
        kind: Kind::Provider,
        reason: "PostgreSQL TIMESTAMPTZ and MySQL/MariaDB DATETIME(6) keep microseconds (SQLx \
                 truncates the nanoseconds); SQLite stores SQLx's RFC 3339 text with nanoseconds.",
    },
    Difference {
        backends: POSTGRES,
        key: "collation.like_ascii_prefix",
        value: r#"["alice"]"#,
        kind: Kind::Provider,
        reason: "PostgreSQL's LIKE is case-sensitive; SQLite's LIKE ignores the case of ASCII \
                 letters (and MySQL/MariaDB's follows its case-insensitive collation).",
    },
    Difference {
        backends: MYSQL_FAMILY,
        key: "collation.eq_same_case",
        value: r#"["Alice","alice"]"#,
        kind: Kind::Provider,
        reason: CASE_INSENSITIVE,
    },
    Difference {
        backends: MYSQL_FAMILY,
        key: "collation.eq_other_case",
        value: r#"["Alice","alice"]"#,
        kind: Kind::Provider,
        reason: CASE_INSENSITIVE,
    },
    Difference {
        backends: MYSQL_FAMILY,
        key: "collation.eq_without_accent",
        value: r#"["Émile","émile"]"#,
        kind: Kind::Provider,
        reason: CASE_INSENSITIVE,
    },
    Difference {
        backends: MYSQL_FAMILY,
        key: "collation.like_accented_prefix",
        value: r#"["Édith","Émile","émile"]"#,
        kind: Kind::Provider,
        reason: CASE_INSENSITIVE,
    },
    Difference {
        backends: MYSQL_FAMILY,
        key: "collation.in_other_case",
        value: r#"["Carl","bob"]"#,
        kind: Kind::Provider,
        reason: CASE_INSENSITIVE,
    },
    Difference {
        backends: MYSQL_FAMILY,
        key: "collation.unique_case_variant",
        value: r#"{"error":"UniqueViolation","constraint":null}"#,
        kind: Kind::Provider,
        reason: "A unique index follows the column collation: MySQL/MariaDB's case-insensitive \
                 default treats `PERSON0@…` and `person0@…` as the same value.",
    },
    Difference {
        backends: &["mariadb"],
        key: "collation.eq_trailing_space",
        value: r#"["bob"]"#,
        kind: Kind::Provider,
        reason: "MariaDB's default collation is PAD SPACE (trailing spaces are ignored in \
                 comparisons); MySQL 8's utf8mb4_0900_ai_ci is NO PAD, like SQLite and PostgreSQL.",
    },
    Difference {
        backends: POSTGRES,
        key: "collation.order_by_name",
        value: r#"["bob","Carl","dora","Édith","_under","Zoe"]"#,
        kind: Kind::Provider,
        reason: LINGUISTIC_ORDER,
    },
    Difference {
        backends: POSTGRES,
        key: "collation.order_by_name_desc",
        value: r#"["Zoe","_under","Édith","dora","Carl","bob"]"#,
        kind: Kind::Provider,
        reason: LINGUISTIC_ORDER,
    },
    Difference {
        backends: MYSQL_FAMILY,
        key: "collation.order_by_name",
        value: r#"["_under","bob","Carl","dora","Édith","Zoe"]"#,
        kind: Kind::Provider,
        reason: LINGUISTIC_ORDER,
    },
    Difference {
        backends: MYSQL_FAMILY,
        key: "collation.order_by_name_desc",
        value: r#"["Zoe","Édith","dora","Carl","bob","_under"]"#,
        kind: Kind::Provider,
        reason: LINGUISTIC_ORDER,
    },
];

pub fn lookup(backend: &str, key: &str) -> Option<&'static Difference> {
    DIFFERENCES
        .iter()
        .find(|difference| difference.key == key && difference.backends.contains(&backend))
}

pub fn keys_for(backend: &str) -> impl Iterator<Item = &'static str> + '_ {
    DIFFERENCES
        .iter()
        .filter(move |difference| difference.backends.contains(&backend))
        .map(|difference| difference.key)
}

/// Every entry parses, names a server backend at most once per step and
/// gives a reason.
pub fn validate() {
    let mut seen = std::collections::BTreeSet::new();
    for difference in DIFFERENCES {
        let _ = difference.value();
        assert!(
            !difference.reason.trim().is_empty(),
            "`{}` has no reason",
            difference.key
        );
        assert!(
            !difference.backends.is_empty(),
            "`{}` names no backend",
            difference.key
        );
        for backend in difference.backends {
            let known = Backend::parse(backend).unwrap_or_else(|| {
                panic!("`{}` names the unknown backend {backend}", difference.key)
            });
            assert_ne!(
                known,
                Backend::Sqlite,
                "`{}`: SQLite is the reference",
                difference.key
            );
            assert!(
                seen.insert((*backend, difference.key)),
                "`{}` is documented twice for {backend}",
                difference.key
            );
        }
        if difference.kind == Kind::Defect {
            println!(
                "open ORM defect at `{}`: {}",
                difference.key, difference.reason
            );
        }
    }
}

//! Per-driver DDL snapshots of the constraint, index and date/time builders.
#![allow(clippy::expect_used)]

use super::{Blueprint, ColumnDefault, ForeignKeyAction};
use crate::Error;

/// Every constraint and index builder on one table.
fn posts() -> Blueprint {
    let mut table = Blueprint::new();
    table.id();
    table.string("slug").not_null().unique().col_type = "VARCHAR(191)".to_string();
    table
        .foreign_id("user_id", "users")
        .on_delete(ForeignKeyAction::Cascade);
    table.integer("editor_id");
    table
        .foreign("editor_id")
        .references("users", "id")
        .on_delete(ForeignKeyAction::SetNull)
        .on_update(ForeignKeyAction::Restrict)
        .name("posts_editor_fk");
    table.integer("position").not_null();
    table.unique(&["user_id", "position"]);
    table.index(&["user_id"]);
    table
        .unique_index(&["editor_id", "position"])
        .name("posts_editor_position");
    table.check("position >= 0").name("posts_position_check");
    table
}

fn create(blueprint: &Blueprint, driver: &str) -> Vec<String> {
    blueprint
        .create_statements("posts", driver, false)
        .expect("create statements")
}

#[test]
fn sqlite_creates_constraints_inline_and_indexes_after_the_table() {
    assert_eq!(
        create(&posts(), "sqlite"),
        [
            "CREATE TABLE IF NOT EXISTS \"posts\" (\n    \
             id INTEGER PRIMARY KEY AUTOINCREMENT,\n    \
             slug VARCHAR(191) NOT NULL UNIQUE,\n    \
             user_id INTEGER NOT NULL,\n    \
             editor_id INTEGER,\n    \
             position INTEGER NOT NULL,\n    \
             CONSTRAINT posts_user_id_position_unique UNIQUE (user_id, position),\n    \
             CONSTRAINT posts_user_id_foreign FOREIGN KEY (user_id) REFERENCES \"users\" (id) ON DELETE CASCADE,\n    \
             CONSTRAINT posts_editor_fk FOREIGN KEY (editor_id) REFERENCES \"users\" (id) ON DELETE SET NULL ON UPDATE RESTRICT,\n    \
             CONSTRAINT posts_position_check CHECK (position >= 0)\n);",
            "CREATE INDEX IF NOT EXISTS posts_user_id_index ON \"posts\" (user_id);",
            "CREATE UNIQUE INDEX IF NOT EXISTS posts_editor_position ON \"posts\" (editor_id, position);",
        ]
    );
}

#[test]
fn postgres_creates_constraints_inline_and_indexes_after_the_table() {
    assert_eq!(
        create(&posts(), "postgres"),
        [
            "CREATE TABLE IF NOT EXISTS \"posts\" (\n    \
             id SERIAL PRIMARY KEY,\n    \
             slug VARCHAR(191) NOT NULL UNIQUE,\n    \
             user_id INTEGER NOT NULL,\n    \
             editor_id INTEGER,\n    \
             position INTEGER NOT NULL,\n    \
             CONSTRAINT posts_user_id_position_unique UNIQUE (user_id, position),\n    \
             CONSTRAINT posts_user_id_foreign FOREIGN KEY (user_id) REFERENCES \"users\" (id) ON DELETE CASCADE,\n    \
             CONSTRAINT posts_editor_fk FOREIGN KEY (editor_id) REFERENCES \"users\" (id) ON DELETE SET NULL ON UPDATE RESTRICT,\n    \
             CONSTRAINT posts_position_check CHECK (position >= 0)\n);",
            "CREATE INDEX IF NOT EXISTS posts_user_id_index ON \"posts\" (user_id);",
            "CREATE UNIQUE INDEX IF NOT EXISTS posts_editor_position ON \"posts\" (editor_id, position);",
        ]
    );
}

#[test]
fn mysql_creates_indexes_inside_create_table() {
    assert_eq!(
        create(&posts(), "mysql"),
        ["CREATE TABLE IF NOT EXISTS `posts` (\n    \
             id INTEGER PRIMARY KEY AUTO_INCREMENT,\n    \
             slug VARCHAR(191) NOT NULL UNIQUE,\n    \
             user_id INTEGER NOT NULL,\n    \
             editor_id INTEGER,\n    \
             position INTEGER NOT NULL,\n    \
             CONSTRAINT posts_user_id_position_unique UNIQUE (user_id, position),\n    \
             CONSTRAINT posts_user_id_foreign FOREIGN KEY (user_id) REFERENCES `users` (id) ON DELETE CASCADE,\n    \
             CONSTRAINT posts_editor_fk FOREIGN KEY (editor_id) REFERENCES `users` (id) ON DELETE SET NULL ON UPDATE RESTRICT,\n    \
             CONSTRAINT posts_position_check CHECK (position >= 0),\n    \
             INDEX posts_user_id_index (user_id),\n    \
             UNIQUE INDEX posts_editor_position (editor_id, position)\n);",]
    );
}

#[test]
fn build_without_a_table_leaves_default_names_to_the_database() {
    let mysql = posts().render(None, "mysql", false).expect("MySQL body");
    assert!(
        mysql.contains(",\n    UNIQUE (user_id, position),"),
        "{mysql}"
    );
    assert!(
        mysql.contains(",\n    FOREIGN KEY (user_id) REFERENCES `users` (id) ON DELETE CASCADE,"),
        "{mysql}"
    );
    assert!(
        mysql.contains("CONSTRAINT posts_editor_fk FOREIGN KEY"),
        "{mysql}"
    );
    assert!(mysql.contains(",\n    INDEX (user_id),"), "{mysql}");
    assert!(
        mysql.ends_with(",\n    UNIQUE INDEX posts_editor_position (editor_id, position)"),
        "{mysql}"
    );
    let sqlite = posts().render(None, "sqlite", false).expect("SQLite body");
    assert!(!sqlite.contains("INDEX"), "{sqlite}");
    assert!(
        sqlite.ends_with("CONSTRAINT posts_position_check CHECK (position >= 0)"),
        "{sqlite}"
    );
}

#[test]
fn mysql_rejects_keys_on_text_columns() {
    let keyed: [fn(&mut Blueprint); 5] = [
        |table| {
            table.string("email").unique();
        },
        |table| {
            table.string("email");
            table.index(&["email"]);
        },
        |table| {
            table.string("email");
            table.unique_index(&["email"]);
        },
        |table| {
            table.string("email");
            table.unique(&["email"]);
        },
        |table| {
            table.string("owner");
            table.foreign("owner").references("users", "email");
        },
    ];
    for declare in keyed {
        let mut table = Blueprint::new();
        declare(&mut table);
        assert!(
            matches!(
                table.create_statements("accounts", "mysql", false),
                Err(Error::Validation(message)) if message.contains("VARCHAR(255)")
            ),
            "MySQL must reject a key on TEXT"
        );
        for driver in ["postgres", "sqlite"] {
            assert!(
                table.create_statements("accounts", driver, false).is_ok(),
                "{driver}"
            );
        }
    }
    // Timestamps are TEXT on the Any pool, DATETIME(6) with strict MySQL.
    let mut table = Blueprint::new();
    table.timestamps();
    table.index(&["created_at"]);
    assert!(table.create_statements("events", "mysql", true).is_err());
    assert!(table.create_statements("events", "mysql", false).is_ok());
}

#[test]
fn constraint_builders_reject_invalid_declarations() {
    let invalid: [fn(&mut Blueprint); 6] = [
        // SET NULL cannot apply to a NOT NULL column.
        |table| {
            table
                .foreign_id("user_id", "users")
                .on_delete(ForeignKeyAction::SetNull);
        },
        |table| {
            table
                .foreign_id("user_id", "users")
                .on_update(ForeignKeyAction::SetNull);
        },
        // A foreign key without a referenced table.
        |table| {
            table.integer("user_id");
            table.foreign("user_id");
        },
        // A default name longer than 63 bytes.
        |table| {
            table.integer("a_really_long_column_name_for_an_index");
            table.integer("another_long_column_name");
            table.index(&[
                "a_really_long_column_name_for_an_index",
                "another_long_column_name",
            ]);
        },
        |table| {
            table.integer("score");
            table.check("score > 0); DROP TABLE posts; --");
        },
        |table| {
            table.integer("score");
            table.unique(&["score; DROP TABLE posts"]);
        },
    ];
    for declare in invalid {
        let mut table = Blueprint::new();
        declare(&mut table);
        for driver in ["sqlite", "postgres", "mysql"] {
            assert!(
                matches!(
                    table.create_statements("posts", driver, false),
                    Err(Error::Validation(_) | Error::Internal(_))
                ),
                "{driver}"
            );
        }
    }
    // An explicit name replaces the over-long default.
    let mut table = Blueprint::new();
    table.integer("a_really_long_column_name_for_an_index");
    table.integer("another_long_column_name");
    table
        .index(&[
            "a_really_long_column_name_for_an_index",
            "another_long_column_name",
        ])
        .name("posts_long_idx");
    assert_eq!(
        table
            .create_statements("posts", "sqlite", false)
            .expect("named")[1],
        "CREATE INDEX IF NOT EXISTS posts_long_idx ON \"posts\" (a_really_long_column_name_for_an_index, another_long_column_name);"
    );
}

/// One column of each date/time type, with and without a current-time default.
fn calendar() -> Blueprint {
    let mut table = Blueprint::new();
    table.date("born_on").not_null();
    table.time("opens_at");
    table.timestamp("local_at");
    table
        .timestamp_tz("paid_at")
        .default(ColumnDefault::CurrentTimestamp);
    table
        .timestamp("seen_at")
        .default(ColumnDefault::CurrentTimestamp);
    table.date("day").default(ColumnDefault::CurrentTimestamp);
    table.time("clock").default(ColumnDefault::CurrentTimestamp);
    table
        .date("launch_on")
        .default(ColumnDefault::Text("2026-01-01".to_string()));
    table
}

#[test]
fn temporal_columns_use_native_types_on_postgres_and_mysql() {
    assert_eq!(
        calendar()
            .render(None, "postgres", false)
            .expect("PostgreSQL"),
        "born_on DATE NOT NULL,\n    \
         opens_at TIME,\n    \
         local_at TIMESTAMP,\n    \
         paid_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,\n    \
         seen_at TIMESTAMP DEFAULT (CURRENT_TIMESTAMP AT TIME ZONE 'UTC'),\n    \
         day DATE DEFAULT (CURRENT_TIMESTAMP AT TIME ZONE 'UTC'),\n    \
         clock TIME DEFAULT (CURRENT_TIMESTAMP AT TIME ZONE 'UTC'),\n    \
         launch_on DATE DEFAULT '2026-01-01'"
    );
    assert_eq!(
        calendar().render(None, "mysql", false).expect("MySQL"),
        "born_on DATE NOT NULL,\n    \
         opens_at TIME(6),\n    \
         local_at DATETIME(6),\n    \
         paid_at DATETIME(6) DEFAULT CURRENT_TIMESTAMP(6),\n    \
         seen_at DATETIME(6) DEFAULT CURRENT_TIMESTAMP(6),\n    \
         day DATE DEFAULT (CURRENT_DATE),\n    \
         clock TIME(6) DEFAULT (CURRENT_TIME(6)),\n    \
         launch_on DATE DEFAULT '2026-01-01'"
    );
}

#[test]
fn temporal_columns_are_sortable_iso_text_on_sqlite() {
    assert_eq!(
        calendar().render(None, "sqlite", false).expect("SQLite"),
        "born_on TEXT NOT NULL,\n    \
         opens_at TEXT,\n    \
         local_at TEXT,\n    \
         paid_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00', 'now')),\n    \
         seen_at TEXT DEFAULT CURRENT_TIMESTAMP,\n    \
         day TEXT DEFAULT CURRENT_DATE,\n    \
         clock TEXT DEFAULT CURRENT_TIME,\n    \
         launch_on TEXT DEFAULT '2026-01-01'"
    );
}

#[test]
fn temporal_column_types_read_their_logical_name_until_built() {
    let mut table = Blueprint::new();
    assert_eq!(table.date("a").col_type, "DATE");
    assert_eq!(table.time("b").col_type, "TIME");
    assert_eq!(table.timestamp("c").col_type, "TIMESTAMP");
    assert_eq!(table.timestamp_tz("d").col_type, "TIMESTAMPTZ");
    // A replaced type is emitted as written on every driver.
    table.timestamp_tz("e").col_type = "TIMESTAMP(3) WITH TIME ZONE".to_string();
    for driver in ["postgres", "mysql", "sqlite"] {
        let sql = table.render(None, driver, false).expect("DDL");
        assert!(
            sql.ends_with("e TIMESTAMP(3) WITH TIME ZONE"),
            "{driver}: {sql}"
        );
    }
}

#[test]
fn the_any_pool_rejects_native_temporal_columns_on_postgres_and_mysql() {
    for driver in ["postgres", "mysql"] {
        assert!(
            matches!(
                calendar().create_statements("events", driver, true),
                Err(Error::Validation(message)) if message.contains("born_on") && message.contains("strict-")
            ),
            "{driver}"
        );
        assert!(
            calendar()
                .create_statements("events", driver, false)
                .is_ok()
        );
        assert!(calendar().alter_statements("events", driver, true).is_err());
    }
    assert!(
        calendar()
            .create_statements("events", "sqlite", true)
            .is_ok()
    );
}

#[test]
fn timestamps_are_utc_instants_on_strict_pools() {
    let mut table = Blueprint::new();
    table.timestamps();
    assert_eq!(table.columns[0].name, "created_at");
    assert_eq!(table.columns[1].name, "updated_at");
    assert_eq!(table.columns[0].col_type, "TIMESTAMPTZ");
    assert!(table.columns.iter().all(|column| column.is_nullable));
    assert_eq!(
        table.render(None, "postgres", false).expect("PostgreSQL"),
        "created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,\n    \
         updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP"
    );
    assert_eq!(
        table.render(None, "mysql", false).expect("MySQL"),
        "created_at DATETIME(6) DEFAULT CURRENT_TIMESTAMP(6),\n    \
         updated_at DATETIME(6) DEFAULT CURRENT_TIMESTAMP(6)"
    );
    assert_eq!(
        table.render(None, "sqlite", false).expect("SQLite"),
        "created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00', 'now')),\n    \
         updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%f+00:00', 'now'))"
    );
}

#[test]
fn timestamps_stay_text_on_the_any_pool() {
    let mut table = Blueprint::new();
    table.timestamps();
    for driver in ["postgres", "sqlite"] {
        assert_eq!(
            table.render(None, driver, true).expect("DDL"),
            "created_at TEXT DEFAULT CURRENT_TIMESTAMP,\n    \
             updated_at TEXT DEFAULT CURRENT_TIMESTAMP",
            "{driver}"
        );
        assert!(table.create_statements("notes", driver, true).is_ok());
    }
    assert_eq!(
        table.render(None, "mysql", true).expect("MySQL"),
        "created_at TEXT DEFAULT (CURRENT_TIMESTAMP),\n    \
         updated_at TEXT DEFAULT (CURRENT_TIMESTAMP)"
    );
    assert!(table.create_statements("notes", "mysql", true).is_ok());
}

//! Blueprint unique constraints, indexes, foreign-key actions, CHECK and
//! `Schema::table` behave alike on every driver.

use rullst_orm::schema::{ForeignKeyAction, Schema};
use rullst_orm::{Error, FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_authors")]
struct Author {
    id: i32,
    email: String,
}

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_books")]
struct Book {
    id: i32,
    author_id: i32,
    editor_id: Option<i32>,
    isbn: String,
    position: i32,
}

/// `contract_books` after `Schema::table` added two columns.
#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_books")]
struct ReviewedBook {
    id: i32,
    author_id: i32,
    editor_id: Option<i32>,
    isbn: String,
    position: i32,
    reviewer_id: Option<i32>,
    slug: Option<String>,
}

/// Names of the indexes the database reports for `table`. SQLite names the
/// index of a table-level UNIQUE constraint itself (`sqlite_autoindex_*`).
async fn index_names(driver: &str, table: &str) -> Vec<String> {
    let sql = match driver {
        "postgres" => "SELECT indexname::TEXT FROM pg_indexes WHERE tablename = $1",
        "mysql" => {
            "SELECT DISTINCT INDEX_NAME FROM information_schema.statistics \
             WHERE table_schema = DATABASE() AND table_name = ?"
        }
        _ => "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = ?",
    };
    let pool = Orm::pool().expect("pool");
    rullst_orm::sqlx::query_as::<_, (String,)>(rullst_orm::sqlx::AssertSqlSafe(sql))
        .bind(table)
        .fetch_all(pool)
        .await
        .unwrap_or_else(|error| panic!("{driver} index catalog: {error}"))
        .into_iter()
        .map(|(name,)| name)
        .collect()
}

fn book(author_id: i32, isbn: &str, position: i32) -> Book {
    Book {
        id: 0,
        author_id,
        editor_id: None,
        isbn: isbn.to_string(),
        position,
    }
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_authors", |table| {
        table.id();
        table.string("email").not_null().unique().col_type = "VARCHAR(191)".to_string();
    })
    .await
    .unwrap_or_else(|error| panic!("{driver} authors: {error}"));
    Schema::create("contract_books", |table| {
        table.id();
        table
            .foreign_id("author_id", "contract_authors")
            .on_delete(ForeignKeyAction::Cascade);
        table.integer("editor_id");
        table
            .foreign("editor_id")
            .references("contract_authors", "id")
            .on_delete(ForeignKeyAction::SetNull);
        table.string("isbn").not_null().col_type = "VARCHAR(32)".to_string();
        table.integer("position").not_null();
        table.unique(&["author_id", "position"]);
        table.unique_index(&["isbn"]);
        table.index(&["editor_id"]);
        table
            .check("position >= 0")
            .name("contract_books_position_check");
    })
    .await
    .unwrap_or_else(|error| panic!("{driver} books: {error}"));

    let indexes = index_names(driver, "contract_books").await;
    for name in [
        "contract_books_isbn_unique",
        "contract_books_editor_id_index",
    ] {
        assert!(
            indexes.iter().any(|index| index == name),
            "{driver}: {indexes:?}"
        );
    }
    if driver != "sqlite" {
        assert!(
            indexes
                .iter()
                .any(|index| index == "contract_books_author_id_position_unique"),
            "{driver}: {indexes:?}"
        );
    }

    let mut ada = Author {
        id: 0,
        email: "ada@example.test".to_string(),
    };
    ada.save().await.expect("insert author");
    let mut grace = Author {
        id: 0,
        email: "grace@example.test".to_string(),
    };
    grace.save().await.expect("insert editor");
    let mut copy = Author {
        id: 0,
        email: "ada@example.test".to_string(),
    };
    let saved = copy.save().await;
    assert!(
        matches!(saved, Err(Error::UniqueViolation { .. })),
        "{driver} column-level unique: {saved:?}"
    );

    let mut first = book(ada.id, "978-0", 1);
    first.editor_id = Some(grace.id);
    first.save().await.expect("insert book");
    let same_position = book(ada.id, "978-1", 1).save().await;
    assert!(
        matches!(&same_position, Err(Error::UniqueViolation { constraint, .. })
            if driver != "postgres" || constraint.as_deref() == Some("contract_books_author_id_position_unique")),
        "{driver} table-level unique: {same_position:?}"
    );
    let same_isbn = book(ada.id, "978-0", 2).save().await;
    assert!(
        matches!(&same_isbn, Err(Error::UniqueViolation { constraint, .. })
            if driver != "postgres" || constraint.as_deref() == Some("contract_books_isbn_unique")),
        "{driver} unique index: {same_isbn:?}"
    );
    let orphan = book(ada.id + grace.id + 1000, "978-2", 1).save().await;
    assert!(
        matches!(&orphan, Err(Error::ForeignKeyViolation { constraint, .. })
            if driver != "postgres" || constraint.as_deref() == Some("contract_books_author_id_foreign")),
        "{driver} missing author: {orphan:?}"
    );
    let negative = book(ada.id, "978-3", -1).save().await;
    assert!(
        negative.is_err(),
        "{driver} CHECK must reject a negative position"
    );

    // Deleting the editor cascades to her own book and clears `editor_id`.
    let mut second = book(grace.id, "978-4", 1);
    second.save().await.expect("insert the editor's own book");
    grace.delete().await.expect("delete the editor");
    assert!(
        Book::find(second.id).await.expect("read book").is_none(),
        "{driver} ON DELETE CASCADE"
    );
    let edited = Book::find(first.id)
        .await
        .expect("read book")
        .expect("book survives its editor");
    assert_eq!(edited.editor_id, None, "{driver} ON DELETE SET NULL");
    ada.delete().await.expect("delete the author");
    assert!(
        Book::find(first.id).await.expect("read book").is_none(),
        "{driver} ON DELETE CASCADE"
    );

    alter(driver).await;

    Schema::drop_if_exists("contract_books")
        .await
        .expect("drop books");
    Schema::drop_if_exists("contract_authors")
        .await
        .expect("drop authors");
}

/// `Schema::table` adds a referencing column, a unique index and an index.
async fn alter(driver: &str) {
    Schema::table("contract_books", |table| {
        table.integer("reviewer_id");
        table
            .foreign("reviewer_id")
            .references("contract_authors", "id")
            .on_delete(ForeignKeyAction::SetNull);
        table.string("slug").col_type = "VARCHAR(64)".to_string();
        table.unique_index(&["slug"]);
        table
            .index(&["reviewer_id", "position"])
            .name("contract_books_reviewer_idx");
    })
    .await
    .unwrap_or_else(|error| panic!("{driver} Schema::table: {error}"));
    let indexes = index_names(driver, "contract_books").await;
    for name in ["contract_books_slug_unique", "contract_books_reviewer_idx"] {
        assert!(
            indexes.iter().any(|index| index == name),
            "{driver}: {indexes:?}"
        );
    }

    let mut author = Author {
        id: 0,
        email: "edsger@example.test".to_string(),
    };
    author.save().await.expect("insert author");
    let reviewed = |reviewer_id, slug: &str, position| ReviewedBook {
        id: 0,
        author_id: author.id,
        editor_id: None,
        isbn: format!("979-{position}"),
        position,
        reviewer_id,
        slug: Some(slug.to_string()),
    };
    let mut book = reviewed(Some(author.id), "notes", 1);
    book.save().await.expect("insert reviewed book");
    let orphan = reviewed(Some(author.id + 1000), "other", 2).save().await;
    assert!(
        matches!(orphan, Err(Error::ForeignKeyViolation { .. })),
        "{driver} added foreign key: {orphan:?}"
    );
    let duplicate = reviewed(None, "notes", 3).save().await;
    assert!(
        matches!(duplicate, Err(Error::UniqueViolation { .. })),
        "{driver} added unique index: {duplicate:?}"
    );

    // SQLite cannot add a CHECK; nothing runs, not even the ADD COLUMN.
    let checked = Schema::table("contract_books", |table| {
        table.integer("pages");
        table.check("pages > 0");
    })
    .await;
    if driver == "sqlite" {
        assert!(
            matches!(&checked, Err(Error::UnsupportedSchemaChange { driver, .. }) if driver == "sqlite"),
            "{checked:?}"
        );
        let column = Orm::raw("SELECT pages FROM contract_books").execute().await;
        assert!(column.is_err(), "the column must not have been added");
    } else {
        checked.unwrap_or_else(|error| panic!("{driver} added CHECK: {error}"));
    }

    // An added SQLite column holds one REFERENCES clause; a second foreign
    // key on it is refused instead of dropped, and nothing runs.
    if driver == "sqlite" {
        let twice = Schema::table("contract_books", |table| {
            table.integer("translator_id");
            table
                .foreign("translator_id")
                .references("contract_authors", "id");
            table
                .foreign("translator_id")
                .references("contract_books", "id");
        })
        .await;
        assert!(
            matches!(&twice, Err(Error::UnsupportedSchemaChange { driver, .. }) if driver == "sqlite"),
            "{twice:?}"
        );
        let column = Orm::raw("SELECT translator_id FROM contract_books")
            .execute()
            .await;
        assert!(column.is_err(), "the column must not have been added");
    }
}

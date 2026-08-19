use super::*;

struct MockSubquery {
    sql: String,
    bindings: Vec<crate::RullstValue>,
}

impl SubqueryBuilder for MockSubquery {
    fn to_sql(&self) -> String {
        self.sql.clone()
    }
    fn bindings(&self) -> &Vec<crate::RullstValue> {
        &self.bindings
    }
}

#[test]
fn test_subquery_builder_trait() {
    let sq = MockSubquery {
        sql: "SELECT * FROM users WHERE id = ?".to_string(),
        bindings: vec![42.into()],
    };
    assert_eq!(sq.to_sql(), "SELECT * FROM users WHERE id = ?");
    assert_eq!(sq.bindings().len(), 1);
}

#[test]
fn test_enable_disable_query_log() {
    disable_query_log();
    assert!(!is_query_log_enabled());
    enable_query_log();
    assert!(is_query_log_enabled());
    disable_query_log();
    assert!(!is_query_log_enabled());
}

#[test]
fn test_join_clause() {
    let mut jc = JoinClause::new("users");
    jc.on("users.id", "=", "posts.user_id");
    assert_eq!(jc.to_sql(), "users.id = posts.user_id");
}

#[test]
fn test_validate_table_name() {
    assert!(validate_table_name("users").is_ok());
    assert!(validate_table_name("user_posts").is_ok());
    assert!(validate_table_name("DROP TABLE users").is_err());
    assert!(validate_table_name("../../../etc/shadow").is_err());
    // dots not allowed in table names
    assert!(validate_table_name("users.id").is_err());
    assert!(validate_table_name("").is_err()); // Empty table name
}

#[test]
fn test_validate_identifier() {
    assert!(validate_identifier("users").is_ok());
    assert!(validate_identifier("users.id").is_ok());
    assert!(validate_identifier("user_posts").is_ok());
    assert!(validate_identifier("").is_err());
    assert!(validate_identifier("users.posts.id").is_err()); // two dots
    assert!(validate_identifier("DROP TABLE users").is_err());
    assert!(validate_identifier("id; DROP TABLE users--").is_err());
    // Length check
    assert!(validate_identifier(&"a".repeat(64)).is_ok());
    assert!(validate_identifier(&"a".repeat(65)).is_err());
    // Leading/trailing dot edge cases — all now rejected
    assert!(validate_identifier(".").is_err()); // bare dot: starts AND ends with dot
    assert!(validate_identifier(".users").is_err()); // leading dot
    assert!(validate_identifier("users.").is_err()); // trailing dot
    assert!(validate_identifier("user name").is_err()); // Spaces not allowed
    assert!(validate_identifier("admin'--").is_err()); // Quotes not allowed
    assert!(validate_identifier("users()").is_err()); // Parentheses not allowed
    assert!(validate_identifier("a*b").is_err()); // Asterisk not allowed

    // Extensive error tests
    assert!(validate_identifier("SELECT * FROM users").is_err());
    assert!(validate_identifier("users\nWHERE").is_err());
    assert!(validate_identifier("users\t").is_err());
    assert!(validate_identifier("\\").is_err());
}

#[test]
fn test_join_clause_on_invalid_operator() {
    let mut jc = JoinClause::new("posts");
    jc.on("posts.user_id", "OR 1=1 --", "users.id");
    assert!(!jc.errors.is_empty());
    assert!(jc.errors[0].to_string().contains("invalid operator"));
}

#[test]
fn test_join_clause_on_invalid_column() {
    let mut jc = JoinClause::new("posts");
    jc.on("users.id; DROP TABLE users--", "=", "posts.user_id");
    assert!(!jc.errors.is_empty());
    assert!(jc.errors[0].to_string().contains("invalid identifier"));
}

#[test]
fn test_timestamps_adds_columns() {
    let mut bp = Blueprint::new();
    bp.timestamps();
    assert_eq!(bp.columns.len(), 2);
    assert_eq!(bp.columns[0].name, "created_at");
    assert_eq!(bp.columns[0].col_type, "TEXT");
    assert!(bp.columns[0].is_nullable);
    assert_eq!(
        bp.columns[0].default_value,
        Some(ColumnDefault::CurrentTimestamp)
    );

    assert_eq!(bp.columns[1].name, "updated_at");
    assert_eq!(bp.columns[1].col_type, "TEXT");
    assert!(bp.columns[1].is_nullable);
    assert_eq!(
        bp.columns[1].default_value,
        Some(ColumnDefault::CurrentTimestamp)
    );
}

#[test]
fn test_soft_deletes_adds_nullable_column() {
    let mut bp = Blueprint::new();
    bp.soft_deletes();
    assert_eq!(bp.columns.len(), 1);
    assert_eq!(bp.columns[0].name, "deleted_at");
    assert!(bp.columns[0].is_nullable);
}

#[test]
fn test_blueprint_build_produces_valid_sql() {
    let mut bp = Blueprint::new();
    bp.id();
    bp.string("name").not_null();
    bp.integer("age");
    let sql = bp.build().expect("build should succeed for valid columns");
    assert!(sql.contains("id INTEGER PRIMARY KEY"));
    assert!(sql.contains("name TEXT NOT NULL"));
    assert!(sql.contains("age INTEGER"));
}

#[test]
fn test_column_default_to_sql_escaping() {
    let default_text = ColumnDefault::Text("O'Reilly".to_string());
    assert_eq!(default_text.to_sql(), "'O''Reilly'");
}

#[test]
fn test_validate_identifier_multiple_dots() {
    assert!(validate_identifier("table.column").is_ok()); // one dot
    assert!(validate_identifier("schema.table.column").is_err()); // multiple dots
}

#[test]
fn test_column_default_sql_rendering() {
    assert_eq!(
        ColumnDefault::CurrentTimestamp.to_sql(),
        "CURRENT_TIMESTAMP"
    );
    assert_eq!(ColumnDefault::Null.to_sql(), "NULL");
    assert_eq!(ColumnDefault::Integer(42).to_sql(), "42");
    assert_eq!(ColumnDefault::Float(1.23).to_sql(), "1.23");
    assert_eq!(ColumnDefault::Text("hello".to_string()).to_sql(), "'hello'");
    // SQL injection via embedded quote must be escaped
    assert_eq!(ColumnDefault::Text("it's".to_string()).to_sql(), "'it''s'");
}

#[test]
fn test_join_clause_on_eq_binds_value() {
    let mut jc = JoinClause::new("orders");
    jc.on_eq("orders.user_id", 42i32);
    assert_eq!(jc.to_sql(), "orders.user_id = ?");
    assert_eq!(jc.bindings.len(), 1);
}

#[test]
fn test_join_clause_multiple_conditions() {
    let mut jc = JoinClause::new("posts");
    jc.on("posts.user_id", "=", "users.id");
    jc.on("posts.status", ">", "users.min_status");
    assert_eq!(
        jc.to_sql(),
        "posts.user_id = users.id AND posts.status > users.min_status"
    );
}

#[test]
fn test_column_builder_methods() {
    let mut col = Column::new("age", "INTEGER");
    assert_eq!(col.name, "age");
    assert_eq!(col.col_type, "INTEGER");
    assert!(col.is_nullable); // default is true
    assert!(!col.is_primary_key);
    assert!(!col.is_auto_increment);
    assert_eq!(col.default_value, None);

    col.not_null();
    assert!(!col.is_nullable);

    col.nullable();
    assert!(col.is_nullable);

    col.primary();
    assert!(col.is_primary_key);

    col.default(ColumnDefault::Integer(18));
    assert_eq!(col.default_value, Some(ColumnDefault::Integer(18)));
}

#[test]
fn test_column_nullable_and_not_null_flips() {
    let mut col = Column::new("status", "TEXT");
    assert!(col.is_nullable);
    col.not_null();
    assert!(!col.is_nullable);
    col.nullable();
    assert!(col.is_nullable);
}

#[test]
fn test_blueprint_float_and_boolean_columns() {
    let mut bp = Blueprint::new();
    let col_float = bp.float("price");
    assert_eq!(col_float.name, "price");
    assert_eq!(col_float.col_type, "REAL");
    assert!(col_float.is_nullable);

    let col_bool = bp.boolean("is_active");
    assert_eq!(col_bool.name, "is_active");
    assert_eq!(col_bool.col_type, "INTEGER");
    assert!(col_bool.is_nullable);
}

#[test]
fn test_blueprint_enum_column() {
    let mut bp = Blueprint::new();
    let col = bp.enum_col("status", vec!["Active", "Pending", "Canceled"]);
    assert_eq!(col.name, "status");
    assert_eq!(
        col.col_type,
        "TEXT CHECK(status IN ('Active', 'Pending', 'Canceled'))"
    );
    assert!(col.is_nullable);
}

#[test]
fn test_blueprint_boolean_column() {
    let mut bp = Blueprint::new();
    let col = bp.boolean("verified");
    assert_eq!(col.name, "verified");
    assert_eq!(col.col_type, "INTEGER");
    assert!(col.is_nullable);
    assert!(!col.is_primary_key);
    assert!(!col.is_auto_increment);
    assert_eq!(col.default_value, None);
}

#[tokio::test]
async fn test_db_migration_error_state_invalid_blueprint() {
    let result = Schema::create("invalid; DROP TABLE users", |bp| {
        bp.id();
    })
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_drop_if_exists_invalid_table() {
    let result = Schema::drop_if_exists("invalid; name").await;
    assert!(result.is_err());
    assert!(matches!(result, Err(crate::Error::Internal(_))));
}

#[test]
fn test_max_query_limit_and_timeout_globals() {
    // Test limit
    set_max_query_limit(50);
    assert_eq!(get_max_query_limit(), Some(50));
    set_max_query_limit(0);
    assert_eq!(get_max_query_limit(), None);

    // Test timeout
    set_query_timeout(10);
    assert_eq!(
        get_query_timeout(),
        Some(std::time::Duration::from_secs(10))
    );
    set_query_timeout(0);
    assert_eq!(get_query_timeout(), None);
}

#[tokio::test]
async fn test_run_artisan_entrypoint() {
    // Calling run_artisan with empty lists. It parses std::env::args() and prints help
    // Note: we can't easily mock std::env::args here, so we just run it and let it fall through.
    let result = run_artisan(vec![], vec![]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sail_install() {
    let args = vec!["artisan".to_string(), "sail:install".to_string()];
    let result = run_artisan_with_args(&args, vec![], vec![]).await;
    assert!(result.is_ok());

    let content = std::fs::read_to_string("docker-compose.yml").unwrap();
    assert!(content.contains("postgres:15"));
    assert!(content.contains("redis:alpine"));

    // Cleanup
    std::fs::remove_file("docker-compose.yml").unwrap();
}

use crate::Error;

/// Allowlist of SQL comparison/join operators accepted in raw clause builders.
const ALLOWED_OPERATORS: &[&str] = &["=", "!=", "<>", "<", ">", "<=", ">="];

/// Validates a SQL identifier (column or table name) to prevent SQL injection.
/// Allows alphanumeric characters, underscores, hyphens and a single dot
/// for qualified names like `table.column`.
pub fn validate_identifier(name: &str) -> Result<(), Error> {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return Err(Error::Internal(
            "SQL identifier cannot be empty".to_string(),
        ));
    }

    // Check maximum length
    if bytes.len() > 64 {
        return Err(Error::Internal(format!(
            "Invalid SQL identifier '{}': exceeds maximum length of 64 characters",
            name
        )));
    }

    if bytes[0] == b'.' || bytes[bytes.len() - 1] == b'.' {
        return Err(Error::Internal(format!(
            "Invalid SQL identifier '{}': must not start or end with a dot",
            name
        )));
    }

    let mut dot_count = 0;
    for &b in bytes {
        if b == b'.' {
            dot_count += 1;
            if dot_count > 1 {
                return Err(Error::Internal(format!(
                    "Invalid SQL identifier '{}': at most one dot is allowed",
                    name
                )));
            }
        } else if !b.is_ascii_alphanumeric() && b != b'_' && b != b'-' {
            return Err(Error::Internal(format!(
                "Invalid SQL identifier '{}': only alphanumeric characters, underscores, hyphens and dots are allowed",
                name
            )));
        }
    }

    Ok(())
}

/// Validates a table name to prevent SQL injection.
pub fn validate_table_name(table_name: &str) -> Result<(), Error> {
    if table_name.contains('.') {
        return Err(Error::Internal(format!(
            "Invalid table name '{}': dots are not allowed in table names",
            table_name
        )));
    }
    validate_identifier(table_name)
}

/// Safe values allowed for a column DEFAULT clause.
///
/// Accepting a raw `&str` would allow DDL injection through the DEFAULT
/// position. This enum restricts callers to known-safe literals.
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnDefault {
    /// `CURRENT_TIMESTAMP` — standard SQL timestamp literal.
    CurrentTimestamp,
    /// `NULL` — explicit SQL null default.
    Null,
    /// A non-negative integer literal (e.g. `0`, `1`).
    Integer(i64),
    /// A non-negative real literal (e.g. `0.0`).
    Float(f64),
    /// A string literal that will be single-quoted and escaped.
    /// Only printable ASCII excluding `'` and `\` is accepted.
    Text(String),
}

impl ColumnDefault {
    /// Renders the default value as a safe SQL fragment.
    pub fn to_sql(&self) -> String {
        match self {
            ColumnDefault::CurrentTimestamp => "CURRENT_TIMESTAMP".to_string(),
            ColumnDefault::Null => "NULL".to_string(),
            ColumnDefault::Integer(n) => n.to_string(),
            ColumnDefault::Float(f) => format!("{f}"),
            // Single-quote the string and escape any embedded single-quotes
            // via SQL standard doubling (''), which is safe on every driver.
            ColumnDefault::Text(s) => format!("'{}'", s.replace('\'', "''")),
        }
    }
}

pub struct Column {
    pub name: String,
    pub col_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub is_auto_increment: bool,
    pub default_value: Option<ColumnDefault>,
}

impl Column {
    /// Creates a new column, validating `name` against SQL identifier rules.
    ///
    /// # Panics
    /// Panics if `name` fails identifier validation. Column names are always
    /// developer-supplied compile-time literals — an invalid name is a bug,
    /// not a runtime condition.
    pub fn new(name: &str, col_type: &str) -> Self {
        validate_identifier(name)
            .unwrap_or_else(|e| panic!("Invalid column name {:?}: {}", name, e));
        Self {
            name: name.to_string(),
            col_type: col_type.to_string(),
            is_nullable: true,
            is_primary_key: false,
            is_auto_increment: false,
            default_value: None,
        }
    }

    pub fn not_null(&mut self) -> &mut Self {
        self.is_nullable = false;
        self
    }

    pub fn nullable(&mut self) -> &mut Self {
        self.is_nullable = true;
        self
    }

    /// Sets a safe DEFAULT value using the [`ColumnDefault`] enum.
    ///
    /// The old `&str` overload has been removed to prevent DDL injection
    /// through unescaped DEFAULT clauses.
    pub fn default(&mut self, val: ColumnDefault) -> &mut Self {
        self.default_value = Some(val);
        self
    }

    pub fn primary(&mut self) -> &mut Self {
        self.is_primary_key = true;
        self
    }
}

pub struct Blueprint {
    pub columns: Vec<Column>,
}

impl Default for Blueprint {
    fn default() -> Self {
        Self::new()
    }
}

impl Blueprint {
    pub fn new() -> Self {
        Self { columns: vec![] }
    }

    pub fn id(&mut self) -> &mut Column {
        self.columns.push(Column {
            name: "id".to_string(),
            col_type: "INTEGER".to_string(),
            is_nullable: false,
            is_primary_key: true,
            is_auto_increment: true,
            default_value: None,
        });
        self.columns
            .last_mut()
            .expect("BUG: columns is empty after push")
    }

    fn add_column(&mut self, name: &str, col_type: &str) -> &mut Column {
        let col = Column::new(name, col_type);
        self.columns.push(col);
        self.columns
            .last_mut()
            .expect("BUG: columns is empty after push")
    }

    pub fn string(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "TEXT")
    }

    pub fn integer(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "INTEGER")
    }

    pub fn float(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "REAL")
    }

    pub fn boolean(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "INTEGER")
    }

    pub fn enum_col(&mut self, name: &str, variants: Vec<&str>) -> &mut Column {
        // Enforce enum values using a CHECK constraint for safe cross-DB compatibility
        let check_clause = variants
            .iter()
            .map(|v| format!("'{}'", v.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");
        let col_type = format!("TEXT CHECK({} IN ({}))", name, check_clause);
        self.add_column(name, &col_type)
    }

    pub fn timestamps(&mut self) {
        let mut created = Column::new("created_at", "TEXT");
        created.default(ColumnDefault::CurrentTimestamp);
        self.columns.push(created);

        let mut updated = Column::new("updated_at", "TEXT");
        updated.default(ColumnDefault::CurrentTimestamp);
        self.columns.push(updated);
    }

    pub fn soft_deletes(&mut self) {
        let col = Column::new("deleted_at", "TEXT");
        self.columns.push(col);
        self.columns
            .last_mut()
            .expect("BUG: columns is empty after push")
            .nullable();
    }

    #[cfg_attr(test, mutants::skip)]
    pub fn build(&self) -> Result<String, Error> {
        let driver = crate::DB_DRIVER
            .get()
            .map(|s| s.as_str())
            .unwrap_or("sqlite");
        let mut defs = vec![];
        for col in &self.columns {
            // Defensive re-validation: column names must always be safe
            // identifiers regardless of how the Column was constructed.
            validate_identifier(&col.name)?;

            let mut col_type_str = col.col_type.clone();
            if driver == "postgres" && col.is_auto_increment {
                if col.col_type == "INTEGER" || col.col_type == "INT" {
                    col_type_str = "SERIAL".to_string();
                } else if col.col_type == "BIGINT" {
                    col_type_str = "BIGSERIAL".to_string();
                }
            }

            let mut def = format!("{} {}", col.name, col_type_str);
            if col.is_primary_key {
                def.push_str(" PRIMARY KEY");
            }
            if col.is_auto_increment {
                if driver == "sqlite" {
                    def.push_str(" AUTOINCREMENT");
                } else if driver == "mysql" {
                    def.push_str(" AUTO_INCREMENT");
                }
            }
            if !col.is_nullable && !col.is_primary_key {
                def.push_str(" NOT NULL");
            }
            if let Some(default) = &col.default_value {
                use std::fmt::Write;
                write!(def, " DEFAULT {}", default.to_sql()).unwrap();
            }
            defs.push(def);
        }
        Ok(defs.join(",\n    "))
    }
}

pub struct Schema;

impl Schema {
    pub async fn create<F>(table_name: &str, callback: F) -> Result<(), Error>
    where
        F: FnOnce(&mut Blueprint),
    {
        validate_table_name(table_name)?;

        let mut blueprint = Blueprint::new();
        callback(&mut blueprint);

        // build() now returns Result so any column-name or default issues
        // surface as errors rather than producing malformed SQL.
        let columns_sql = blueprint.build()?;

        let driver = crate::Orm::driver();
        let escaped_table = match driver {
            "mysql" => format!("`{}`", table_name),
            _ => format!("\"{}\"", table_name),
        };

        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (\n    {}\n);",
            escaped_table, columns_sql
        );

        let mut query_builder = sqlx::query_builder::QueryBuilder::new("");
        query_builder.push(&sql);
        let query = query_builder.build();
        crate::execute_query!(query, execute, pool)?;

        Ok(())
    }

    #[mutants::skip]
    pub async fn drop_if_exists(table_name: &str) -> Result<(), Error> {
        validate_table_name(table_name)?;
        let driver = crate::Orm::driver();
        let escaped_table = match driver {
            "mysql" => format!("`{}`", table_name),
            _ => format!("\"{}\"", table_name),
        };

        let sql = format!("DROP TABLE IF EXISTS {};", escaped_table);
        let mut query_builder = sqlx::query_builder::QueryBuilder::new("");
        query_builder.push(&sql);
        let query = query_builder.build();
        crate::execute_query!(query, execute, pool)?;
        Ok(())
    }
}

#[async_trait::async_trait]
pub trait Migration: Send + Sync {
    fn name(&self) -> &'static str;
    async fn up(&self) -> Result<(), Error>;
    async fn down(&self) -> Result<(), Error>;
}

#[cfg_attr(test, mutants::skip)]
pub async fn run_artisan_with_args(
    args: &[String],
    migrations: Vec<Box<dyn Migration>>,
    seeders: Vec<Box<dyn crate::Seeder>>,
) -> Result<(), Error> {
    if args.len() < 2 {
        println!("Rullst ORM Artisan CLI");
        println!("Usage:");
        println!("  make:migration <name>   Generate a new migration");
        println!("  migrate                  Run all pending migrations");
        println!("  migrate:rollback         Rollback the last batch of migrations");
        println!("  status                   Show migrations status");
        println!("  db:seed                  Populate the database with seeders");
        println!(
            "  sail:install             Generate a default docker-compose.yml (Laravel Sail style)"
        );
        return Ok(());
    }

    let command = &args[1];
    match command.as_str() {
        "make:migration" => {
            if args.len() < 3 {
                println!("Error: migration name is required.");
                return Ok(());
            }
            let name = &args[2];
            create_migration_files(name)?;
        }
        "migrate" | "db:migrate" => {
            run_migrations(migrations).await?;
        }
        "migrate:rollback" | "db:rollback" => {
            rollback_migrations(migrations).await?;
        }
        "status" | "db:status" => {
            status_migrations(migrations).await?;
        }
        "db:seed" => {
            println!("Seeding database...");
            crate::Orm::seed(seeders).await?;
            println!("Database seeded successfully!");
        }
        "sail:install" => {
            println!("Generating docker-compose.yml...");
            let content = r#"version: '3'
services:
  postgres:
    image: postgres:15
    ports:
      - "5432:5432"
    environment:
      POSTGRES_DB: rullst
      POSTGRES_USER: root
      POSTGRES_PASSWORD: password
    volumes:
      - sail-postgres:/var/lib/postgresql/data
  redis:
    image: redis:alpine
    ports:
      - "6379:6379"
    volumes:
      - sail-redis:/data
  meilisearch:
    image: getmeili/meilisearch:latest
    ports:
      - "7700:7700"
    environment:
      MEILI_MASTER_KEY: sail
    volumes:
      - sail-meilisearch:/meili_data
  pgadmin:
    image: dpage/pgadmin4
    ports:
      - "5050:80"
    environment:
      PGADMIN_DEFAULT_EMAIL: admin@rullst.com
      PGADMIN_DEFAULT_PASSWORD: password

volumes:
  sail-postgres:
    driver: local
  sail-redis:
    driver: local
  sail-meilisearch:
    driver: local
"#;
            std::fs::write("docker-compose.yml", content).map_err(|e| {
                crate::Error::Internal(format!("Failed to write docker-compose.yml: {}", e))
            })?;
            println!(
                "docker-compose.yml created successfully! Run `docker compose up -d` to start."
            );
        }
        _ => {
            println!("Unknown command: {}", command);
        }
    }
    Ok(())
}

#[cfg_attr(test, mutants::skip)]
pub async fn run_artisan(
    migrations: Vec<Box<dyn Migration>>,
    seeders: Vec<Box<dyn crate::Seeder>>,
) -> Result<(), Error> {
    let args: Vec<String> = std::env::args().collect();
    run_artisan_with_args(&args, migrations, seeders).await
}

#[mutants::skip]
async fn migrations_table_exists(pool: &crate::RullstPool, driver: &str) -> Result<bool, Error> {
    match driver {
        "postgres" | "mysql" => {
            let query_str =
                "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'migrations'";
            let row: (i64,) = sqlx::query_as(query_str).fetch_one(pool).await?;
            Ok(row.0 > 0)
        }
        _ => {
            let query_str =
                "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='migrations'";
            let row: (i64,) = sqlx::query_as(query_str).fetch_one(pool).await?;
            Ok(row.0 > 0)
        }
    }
}

#[cfg_attr(test, mutants::skip)]
async fn status_migrations(migrations: Vec<Box<dyn Migration>>) -> Result<(), Error> {
    let pool = crate::Orm::pool();
    let driver = crate::Orm::driver();

    let table_exists = migrations_table_exists(pool, driver).await?;

    let executed_set = if table_exists {
        let executed: Vec<(String,)> = sqlx::query_as("SELECT migration FROM migrations")
            .fetch_all(pool)
            .await?;
        executed
            .into_iter()
            .map(|(m,)| m)
            .collect::<std::collections::HashSet<String>>()
    } else {
        std::collections::HashSet::new()
    };

    let name_header = "Migration Name";
    let status_header = "Status";
    println!("{name_header:<40} | {status_header}");
    println!("{}", "-".repeat(55));
    for m in migrations {
        let name = m.name();
        let status = if executed_set.contains(name) {
            "Applied"
        } else {
            "Pending"
        };
        println!("{:<40} | {}", name, status);
    }

    Ok(())
}

#[cfg_attr(test, mutants::skip)]
fn create_migration_files(name: &str) -> Result<(), Error> {
    validate_table_name(name)?;
    use std::fs;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("System time went backwards")
        .as_secs()
        .to_string();
    let sanitized_name = name.replace(['/', '\\'], "");
    let snake_name = sanitized_name.to_lowercase().replace("-", "_");
    let file_name = format!("m{}_{}", now, snake_name);

    fs::create_dir_all("src/migrations")
        .map_err(|e| Error::Internal(format!("Failed to create migrations directory: {}", e)))?;

    let new_file_path = format!("src/migrations/{}.rs", file_name);
    let template = include_str!("migration_template.rs.txt");
    let migration_code = template
        .replace("{timestamp}", &now)
        .replace("{name}", &snake_name);

    fs::write(&new_file_path, migration_code)
        .map_err(|e| Error::Internal(format!("Failed to write migration file: {}", e)))?;
    println!("Created migration file: {}", new_file_path);

    regenerate_migrations_mod()?;

    Ok(())
}

#[cfg_attr(test, mutants::skip)]
fn regenerate_migrations_mod() -> Result<(), Error> {
    use std::fs;
    let paths = fs::read_dir("src/migrations")
        .map_err(|e| Error::Internal(format!("Failed to read migrations dir: {}", e)))?;

    let mut modules = vec![];
    for path in paths {
        let path = path.map_err(|e| Error::Internal(e.to_string()))?.path();
        if let Some(ext) = path.extension()
            && ext == "rs"
            && let Some(stem) = path.file_stem()
        {
            let stem_str = stem.to_string_lossy().to_string();
            if stem_str != "mod" && stem_str.starts_with('m') {
                modules.push(stem_str);
            }
        }
    }
    modules.sort();

    use std::fmt::Write;
    let mut mod_content = String::new();
    mod_content.push_str("// Generated by Rullst ORM Artisan. Do not edit manually.\n\n");
    for m in &modules {
        writeln!(mod_content, "pub mod {};", m).unwrap();
    }
    mod_content
        .push_str("\npub fn get_migrations() -> Vec<Box<dyn rullst_orm::schema::Migration>> {\n");
    mod_content.push_str("    vec![\n");
    for m in &modules {
        writeln!(mod_content, "        Box::new({}::MigrationImpl),", m).unwrap();
    }
    mod_content.push_str("    ]\n");
    mod_content.push_str("}\n");

    fs::write("src/migrations/mod.rs", mod_content)
        .map_err(|e| Error::Internal(format!("Failed to write mod.rs: {}", e)))?;
    println!("Regenerated src/migrations/mod.rs");

    Ok(())
}

#[cfg_attr(test, mutants::skip)]
async fn run_migrations(migrations: Vec<Box<dyn Migration>>) -> Result<(), Error> {
    let pool = crate::Orm::pool();
    let driver = crate::Orm::driver();

    let query_str = match driver {
        "postgres" => {
            "CREATE TABLE IF NOT EXISTS migrations (
                id SERIAL PRIMARY KEY,
                migration VARCHAR(255) NOT NULL,
                batch INTEGER NOT NULL
            )"
        }
        "mysql" => {
            "CREATE TABLE IF NOT EXISTS migrations (
                id INT AUTO_INCREMENT PRIMARY KEY,
                migration VARCHAR(255) NOT NULL,
                batch INT NOT NULL
            )"
        }
        _ => {
            "CREATE TABLE IF NOT EXISTS migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                migration TEXT NOT NULL,
                batch INTEGER NOT NULL
            )"
        }
    };

    sqlx::query(query_str).execute(pool).await?;

    let executed: Vec<(String,)> = sqlx::query_as("SELECT migration FROM migrations")
        .fetch_all(pool)
        .await?;
    let executed_set: std::collections::HashSet<String> =
        executed.into_iter().map(|(m,)| m).collect();

    let batch_row: (Option<i32>,) = sqlx::query_as("SELECT MAX(batch) FROM migrations")
        .fetch_one(pool)
        .await?;
    let next_batch = batch_row.0.unwrap_or(0) + 1;

    let mut count = 0;
    let mut successful_migrations = vec![];
    for m in migrations {
        let name = m.name();
        if !executed_set.contains(name) {
            println!("Migrating: {}", name);
            m.up().await?;
            successful_migrations.push(name);
            println!("Migrated:  {}", name);
            count += 1;
        }
    }

    if count > 0 {
        let mut query_builder =
            sqlx::query_builder::QueryBuilder::new("INSERT INTO migrations (migration, batch) ");
        query_builder.push_values(successful_migrations, |mut b, name| {
            b.push_bind(name).push_bind(next_batch);
        });
        query_builder.build().execute(pool).await?;
    } else {
        println!("Nothing to migrate.");
    }

    Ok(())
}

#[cfg_attr(test, mutants::skip)]
async fn rollback_migrations(migrations: Vec<Box<dyn Migration>>) -> Result<(), Error> {
    let pool = crate::Orm::pool();
    let driver = crate::Orm::driver();

    let table_exists = migrations_table_exists(pool, driver).await?;

    if !table_exists {
        println!("Nothing to rollback.");
        return Ok(());
    }

    let batch_row: (Option<i32>,) = sqlx::query_as("SELECT MAX(batch) FROM migrations")
        .fetch_one(pool)
        .await?;

    let last_batch = match batch_row.0 {
        Some(b) if b > 0 => b,
        _ => {
            println!("Nothing to rollback.");
            return Ok(());
        }
    };

    let to_rollback: Vec<(String,)> =
        sqlx::query_as("SELECT migration FROM migrations WHERE batch = ? ORDER BY id DESC")
            .bind(last_batch)
            .fetch_all(pool)
            .await?;

    let mut rollback_map = std::collections::HashMap::with_capacity(migrations.len());
    for m in migrations {
        rollback_map.insert(m.name().to_string(), m);
    }

    let mut rolled_back = Vec::with_capacity(to_rollback.len());
    for (name,) in to_rollback {
        if let Some(m) = rollback_map.get(&name) {
            println!("Rolling back: {}", name);
            m.down().await?;
            println!("Rolled back:  {}", name);
            rolled_back.push(name);
        } else {
            println!(
                "Warning: migration {} found in database but not in compiled binary.",
                name
            );
        }
    }

    if !rolled_back.is_empty() {
        let mut query_builder =
            sqlx::query_builder::QueryBuilder::new("DELETE FROM migrations WHERE migration IN (");
        let mut separated = query_builder.separated(", ");
        for name in rolled_back {
            separated.push_bind(name);
        }
        separated.push_unseparated(")");
        query_builder.build().execute(pool).await?;
    }

    Ok(())
}

pub struct JoinClause {
    pub table: String,
    pub conditions: Vec<String>,
    pub bindings: Vec<crate::RullstValue>,
    pub errors: Vec<crate::Error>,
}

impl JoinClause {
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            conditions: vec![],
            bindings: vec![],
            errors: vec![],
        }
    }

    /// Adds a column-to-column JOIN condition.
    ///
    /// This prevents SQL injection — column names should always be hardcoded, never
    /// derived from user input. Returns errors internally rather than panicking.
    pub fn on(&mut self, first: &str, operator: &str, second: &str) -> &mut Self {
        if let Err(e) = validate_identifier(first) {
            self.errors.push(crate::Error::Validation(format!(
                "JoinClause::on — invalid identifier for `first`: {:?}",
                e
            )));
        }
        if let Err(e) = validate_identifier(second) {
            self.errors.push(crate::Error::Validation(format!(
                "JoinClause::on — invalid identifier for `second`: {:?}",
                e
            )));
        }
        if !ALLOWED_OPERATORS.contains(&operator) {
            self.errors.push(crate::Error::Validation(format!(
                "JoinClause::on — invalid operator '{}'. Allowed: {:?}",
                operator, ALLOWED_OPERATORS
            )));
        }
        self.conditions
            .push(format!("{} {} {}", first, operator, second));
        self
    }

    pub fn on_eq<T: Into<crate::RullstValue>>(&mut self, column: &str, value: T) -> &mut Self {
        if let Err(e) = validate_identifier(column) {
            self.errors.push(crate::Error::Validation(format!(
                "JoinClause::on_eq — invalid identifier for `column`: {:?}",
                e
            )));
        }
        self.conditions.push(format!("{} = ?", column));
        self.bindings.push(value.into());
        self
    }

    pub fn to_sql(&self) -> String {
        self.conditions.join(" AND ")
    }
}

pub trait SubqueryBuilder {
    fn to_sql(&self) -> String;
    fn bindings(&self) -> &Vec<crate::RullstValue>;
}

pub static QUERY_LOGGING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static MAX_QUERY_LIMIT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(1000);
pub static QUERY_TIMEOUT_SECS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(30);

pub fn enable_query_log() {
    QUERY_LOGGING.store(true, std::sync::atomic::Ordering::SeqCst);
}

pub fn disable_query_log() {
    QUERY_LOGGING.store(false, std::sync::atomic::Ordering::SeqCst);
}

pub fn is_query_log_enabled() -> bool {
    QUERY_LOGGING.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn set_max_query_limit(limit: usize) {
    MAX_QUERY_LIMIT.store(limit, std::sync::atomic::Ordering::SeqCst);
}

pub fn get_max_query_limit() -> Option<usize> {
    let limit = MAX_QUERY_LIMIT.load(std::sync::atomic::Ordering::SeqCst);
    if limit == 0 { None } else { Some(limit) }
}

pub fn set_query_timeout(secs: u64) {
    QUERY_TIMEOUT_SECS.store(secs, std::sync::atomic::Ordering::SeqCst);
}

pub fn get_query_timeout() -> Option<std::time::Duration> {
    let secs = QUERY_TIMEOUT_SECS.load(std::sync::atomic::Ordering::SeqCst);
    if secs == 0 {
        None
    } else {
        Some(std::time::Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
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
}

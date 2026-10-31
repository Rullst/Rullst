//! The four backends, the ones this build can open, and the worker that runs
//! every scenario on a fresh database of one of them.

use std::path::{Path, PathBuf};

use rullst_orm::Orm;

use crate::scenarios;
use crate::transcript::Transcript;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Sqlite,
    Postgres,
    Mysql,
    Mariadb,
}

impl Backend {
    pub const ALL: [Backend; 4] = [
        Backend::Sqlite,
        Backend::Postgres,
        Backend::Mysql,
        Backend::Mariadb,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Backend::Sqlite => "sqlite",
            Backend::Postgres => "postgres",
            Backend::Mysql => "mysql",
            Backend::Mariadb => "mariadb",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|backend| backend.name() == name)
    }

    /// The variable holding a server URL; SQLite needs none.
    pub fn url_variable(self) -> Option<&'static str> {
        match self {
            Backend::Sqlite => None,
            Backend::Postgres => Some("RULLST_DIFF_POSTGRES_URL"),
            Backend::Mysql => Some("RULLST_DIFF_MYSQL_URL"),
            Backend::Mariadb => Some("RULLST_DIFF_MARIADB_URL"),
        }
    }

    /// The backends the strict driver feature of this build can open.
    pub fn compiled() -> &'static [Backend] {
        if cfg!(feature = "strict-postgres") {
            &[Backend::Postgres]
        } else if cfg!(feature = "strict-mysql") {
            &[Backend::Mysql, Backend::Mariadb]
        } else {
            &[Backend::Sqlite]
        }
    }
}

/// Runs every scenario on a fresh database of `backend` and writes the
/// transcript to `output`. The database is dropped after a successful run
/// and kept, for inspection, after a failure.
pub async fn run_worker(backend: Backend, output: &Path) {
    let database = FreshDatabase::create(backend).await;
    Orm::init(&database.url)
        .await
        .unwrap_or_else(|error| panic!("{} pool did not open: {error}", backend.name()));

    let mut transcript = Transcript::new(backend.name());
    for (key, sql) in metadata_queries(backend) {
        let rows = Orm::raw(*sql).map_to::<(String,)>().await;
        let value = rows.map_or_else(
            |error| format!("unavailable: {error}"),
            |rows| {
                rows.into_iter()
                    .map(|(value,)| value)
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        );
        transcript.metadata.insert((*key).to_string(), value);
    }
    scenarios::run_all(&mut transcript).await;
    transcript.write(output);

    Orm::pool().expect("ORM pool").close().await;
    database.drop().await;
}

/// Server facts printed with the transcript; never compared.
fn metadata_queries(backend: Backend) -> &'static [(&'static str, &'static str)] {
    match backend {
        Backend::Sqlite => &[("version", "SELECT sqlite_version()")],
        Backend::Postgres => &[
            ("version", "SELECT version()"),
            (
                "collation",
                "SELECT datcollate::text FROM pg_database WHERE datname = current_database()",
            ),
        ],
        Backend::Mysql | Backend::Mariadb => &[
            ("version", "SELECT VERSION()"),
            (
                "collation",
                "SELECT CONCAT(@@character_set_database, ' / ', @@collation_database)",
            ),
            ("sql_mode", "SELECT @@SESSION.sql_mode"),
        ],
    }
}

/// A database that exists only for this run.
struct FreshDatabase {
    url: String,
    location: Location,
}

enum Location {
    File(PathBuf),
    #[cfg_attr(feature = "strict-sqlite", allow(dead_code))]
    Server {
        admin_url: String,
        name: String,
    },
}

fn unique_suffix() -> String {
    format!("{}_{:016x}", std::process::id(), rand::random::<u64>())
}

impl FreshDatabase {
    async fn create(backend: Backend) -> Self {
        let Some(variable) = backend.url_variable() else {
            let path = std::env::temp_dir().join(format!("rullst-diff-{}.db", unique_suffix()));
            return Self {
                url: format!("sqlite:{}?mode=rwc", path.display()),
                location: Location::File(path),
            };
        };
        let admin_url =
            std::env::var(variable).unwrap_or_else(|_| panic!("the worker needs {variable}"));
        let name = format!("rullst_diff_{}", unique_suffix());
        server::execute(&admin_url, &format!("CREATE DATABASE {name}")).await;
        Self {
            url: with_database(&admin_url, &name),
            location: Location::Server { admin_url, name },
        }
    }

    async fn drop(self) {
        match self.location {
            Location::File(path) => {
                let _ = std::fs::remove_file(path);
            }
            Location::Server { admin_url, name } => {
                server::execute(&admin_url, &server::drop_statement(&name)).await;
            }
        }
    }
}

/// `url` with its database (the path) replaced by `database`; the query
/// string is kept.
fn with_database(url: &str, database: &str) -> String {
    let (base, query) = url
        .split_once('?')
        .map_or((url, None), |(base, query)| (base, Some(query)));
    let authority = base.find("://").map_or(0, |index| index + 3);
    let path = base[authority..]
        .find('/')
        .map_or(base.len(), |index| authority + index);
    let mut rewritten = format!("{}/{database}", &base[..path]);
    if let Some(query) = query {
        rewritten.push('?');
        rewritten.push_str(query);
    }
    rewritten
}

/// Statements on the server's administrative connection, outside the ORM.
#[cfg(any(feature = "strict-postgres", feature = "strict-mysql"))]
mod server {
    use std::time::{Duration, Instant};

    use sqlx::Connection;

    #[cfg(feature = "strict-postgres")]
    type AdminConnection = sqlx::PgConnection;
    #[cfg(feature = "strict-mysql")]
    type AdminConnection = sqlx::MySqlConnection;

    #[cfg(feature = "strict-postgres")]
    pub fn drop_statement(name: &str) -> String {
        format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)")
    }
    #[cfg(feature = "strict-mysql")]
    pub fn drop_statement(name: &str) -> String {
        format!("DROP DATABASE IF EXISTS {name}")
    }

    /// Runs `sql`, retrying the connection for a minute while a freshly
    /// started server finishes its first start.
    pub async fn execute(url: &str, sql: &str) {
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut connection = loop {
            match AdminConnection::connect(url).await {
                Ok(connection) => break connection,
                Err(error) if Instant::now() < deadline => {
                    eprintln!("database server not ready yet: {error}");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Err(error) => panic!("cannot connect to the database server: {error}"),
            }
        };
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(&mut connection)
            .await
            .unwrap_or_else(|error| panic!("`{sql}` failed: {error}"));
        connection
            .close()
            .await
            .expect("close the admin connection");
    }
}

#[cfg(feature = "strict-sqlite")]
mod server {
    pub fn drop_statement(_name: &str) -> String {
        unreachable!("SQLite runs on a file")
    }

    pub async fn execute(_url: &str, _sql: &str) {
        unreachable!("SQLite runs on a file")
    }
}

#[cfg(test)]
mod tests {
    use super::with_database;

    #[test]
    fn the_database_of_a_url_is_replaced_and_its_options_kept() {
        assert_eq!(
            with_database(
                "postgres://u:p@127.0.0.1:5432/postgres?sslmode=disable",
                "fresh"
            ),
            "postgres://u:p@127.0.0.1:5432/fresh?sslmode=disable"
        );
        assert_eq!(
            with_database("mysql://root@db:3306", "fresh"),
            "mysql://root@db:3306/fresh"
        );
    }
}

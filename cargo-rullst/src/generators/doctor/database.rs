//! Database reachability and migration state. Server databases get a bounded
//! TCP connect (no credentials are sent); a local SQLite file is opened
//! read-only to compare applied migrations with `src/migrations`. URLs are
//! reduced to host and port before anything is displayed.

use super::context::{ProjectContext, Source, Vars, lookup};
use super::report::Check;
use crate::ui::home::display_safe;
use std::collections::BTreeSet;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const QUERY_TIMEOUT: Duration = Duration::from_secs(3);
const MIGRATION_ENTRY_LIMIT: usize = 10_000;

/// What a database URL points to, without its credentials.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    SqliteMemory,
    SqliteFile(PathBuf),
    Server {
        kind: &'static str,
        host: String,
        port: u16,
    },
    /// A remote or mock endpoint the doctor does not contact.
    NotProbed(&'static str),
}

/// Parses `url` far enough to probe it; credentials are discarded.
pub(crate) fn parse_target(url: &str) -> Target {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("mock") {
        return Target::NotProbed("mock development fallback");
    }
    if let Some(rest) = lower
        .strip_prefix("sqlite://")
        .or_else(|| lower.strip_prefix("sqlite:"))
    {
        let original = url.get(url.len() - rest.len()..).unwrap_or(rest);
        let path = original.split('?').next().unwrap_or_default();
        if path.is_empty() || path == ":memory:" || lower.contains("mode=memory") {
            return Target::SqliteMemory;
        }
        return Target::SqliteFile(PathBuf::from(path));
    }
    let Some((scheme, rest)) = url.split_once("://") else {
        return Target::NotProbed("unrecognized URL");
    };
    let (kind, default_port) = match scheme.to_ascii_lowercase().as_str() {
        "postgres" | "postgresql" => ("PostgreSQL", 5432),
        "mysql" | "mariadb" => ("MySQL/MariaDB", 3306),
        "redis" | "rediss" => ("Redis", 6379),
        "libsql" | "http" | "https" | "wss" | "ws" => {
            return Target::NotProbed("remote endpoint");
        }
        _ => return Target::NotProbed("unsupported scheme"),
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let (host, port) = if let Some(bracketed) = host_port.strip_prefix('[') {
        let (host, after) = bracketed.split_once(']').unwrap_or((bracketed, ""));
        (
            host.to_string(),
            after.strip_prefix(':').unwrap_or_default(),
        )
    } else {
        match host_port.rsplit_once(':') {
            Some((host, port)) => (host.to_string(), port),
            None => (host_port.to_string(), ""),
        }
    };
    if host.is_empty() {
        return Target::NotProbed("socket or host parameter");
    }
    let port = if port.is_empty() {
        default_port
    } else {
        match port.parse() {
            Ok(port) => port,
            Err(_) => return Target::NotProbed("invalid port"),
        }
    };
    Target::Server {
        kind,
        host: display_safe(&host),
        port,
    }
}

/// Connects to `host:port`; the error text names the cause, never a URL.
pub(crate) fn tcp_reachable(host: &str, port: u16) -> Result<(), String> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|_| "the host name could not be resolved".to_string())?;
    let mut last = "no address".to_string();
    for address in addresses.take(4) {
        match TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) {
            Ok(_) => return Ok(()),
            Err(error) => {
                last = match error.kind() {
                    std::io::ErrorKind::ConnectionRefused => "connection refused".to_string(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => {
                        "connection timed out".to_string()
                    }
                    _ => crate::ui::error_report::sanitize(&error.to_string()),
                }
            }
        }
    }
    Err(last)
}

/// The primary database URL with the runtime precedence: environment,
/// `.env`, `[database].url` in `Rullst.toml`, then Turso's variable.
fn primary_url(project: &ProjectContext, vars: &impl Vars) -> Option<(String, Source)> {
    if let Some(found) = lookup(project, vars, "DATABASE_URL") {
        return Some(found);
    }
    let configured = project
        .rullst_toml
        .as_deref()
        .and_then(|contents| crate::generators::dev::parse_rullst_toml(contents).ok())
        .and_then(|config| {
            config
                .get("database")
                .and_then(|database| database.get("url"))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        });
    if let Some(url) = configured {
        return Some((url, Source::RullstToml));
    }
    lookup(project, vars, "TURSO_DATABASE_URL").filter(|(value, _)| !value.trim().is_empty())
}

fn resolve(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn server_check(
    id: &'static str,
    title: &'static str,
    target: &Target,
    source: Source,
    failing: bool,
) -> Check {
    let Target::Server { kind, host, port } = target else {
        return Check::info(id, title, "not probed");
    };
    let fix = format!(
        "Start the server (for example `docker compose up -d`) or fix the URL in {}",
        source.label()
    );
    match tcp_reachable(host, *port) {
        Ok(()) => Check::pass(id, title, format!("{kind} reachable at {host}:{port}")),
        Err(reason) if failing => {
            Check::fail(id, title, format!("{kind} at {host}:{port}: {reason}"), fix)
        }
        Err(reason) => Check::warn(id, title, format!("{kind} at {host}:{port}: {reason}"), fix),
    }
}

fn primary_check(project: &ProjectContext, vars: &impl Vars) -> Check {
    const ID: &str = "database.primary";
    const TITLE: &str = "Primary database";
    let Some((url, source)) = primary_url(project, vars) else {
        return Check::info(ID, TITLE, "no DATABASE_URL configured");
    };
    match parse_target(&url) {
        Target::SqliteMemory => Check::info(ID, TITLE, "in-memory SQLite (nothing to reach)"),
        Target::SqliteFile(path) => {
            let full = resolve(&project.root, &path);
            let shown = display_safe(&path.to_string_lossy());
            match std::fs::metadata(&full) {
                Ok(metadata) if metadata.is_file() => Check::pass(
                    ID,
                    TITLE,
                    format!(
                        "SQLite file {shown} ({}) from {}",
                        human_bytes(metadata.len()),
                        source.label()
                    ),
                ),
                Ok(_) => Check::fail(
                    ID,
                    TITLE,
                    format!("SQLite path {shown} is not a file"),
                    format!(
                        "Point DATABASE_URL in {} at a database file",
                        source.label()
                    ),
                ),
                Err(_) if full.parent().is_some_and(Path::is_dir) => Check::info(
                    ID,
                    TITLE,
                    format!(
                        "SQLite file {shown} does not exist yet; cargo rullst db:migrate creates it"
                    ),
                ),
                Err(_) => Check::fail(
                    ID,
                    TITLE,
                    format!("the directory for SQLite file {shown} does not exist"),
                    format!(
                        "Create the directory or fix DATABASE_URL in {}",
                        source.label()
                    ),
                ),
            }
        }
        target @ Target::Server { .. } => server_check(ID, TITLE, &target, source, true),
        Target::NotProbed(reason) => Check::info(ID, TITLE, format!("{reason} (not probed)")),
    }
}

fn redis_check(project: &ProjectContext, vars: &impl Vars) -> Option<Check> {
    let (url, source) =
        lookup(project, vars, "REDIS_URL").filter(|(value, _)| !value.trim().is_empty())?;
    let target = parse_target(&url);
    Some(match target {
        Target::Server { .. } => server_check("database.redis", "Redis", &target, source, false),
        Target::NotProbed(reason) => {
            Check::info("database.redis", "Redis", format!("{reason} (not probed)"))
        }
        _ => Check::info("database.redis", "Redis", "not a Redis URL (not probed)"),
    })
}

pub(crate) fn database_checks(project: &ProjectContext, vars: &impl Vars) -> Vec<Check> {
    let mut checks = vec![primary_check(project, vars)];
    checks.extend(redis_check(project, vars));
    checks
}

/// `src/migrations/m*.rs` file stems (the registered migration names).
pub(crate) fn migration_names(root: &Path) -> Option<BTreeSet<String>> {
    let entries = std::fs::read_dir(root.join("src").join("migrations")).ok()?;
    Some(
        entries
            .take(MIGRATION_ENTRY_LIMIT)
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name().to_str()?.to_string();
                let stem = name.strip_suffix(".rs")?;
                (stem.starts_with('m') && stem != "mod").then(|| stem.to_string())
            })
            .collect(),
    )
}

/// Applied migration names read from a SQLite file opened read-only.
fn applied_sqlite(path: &Path) -> Result<BTreeSet<String>, String> {
    use sqlx::ConnectOptions;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    runtime.block_on(async {
        let query = async {
            let mut connection = sqlx::sqlite::SqliteConnectOptions::new()
                .filename(path)
                .read_only(true)
                .create_if_missing(false)
                .connect()
                .await?;
            sqlx::query_scalar::<_, String>("SELECT migration FROM migrations")
                .fetch_all(&mut connection)
                .await
        };
        match tokio::time::timeout(QUERY_TIMEOUT, query).await {
            Ok(Ok(names)) => Ok(names.into_iter().collect()),
            Ok(Err(error)) => Err(error.to_string()),
            Err(_) => Err("timed out".to_string()),
        }
    })
}

/// Compares migration files with the applied names.
pub(crate) fn pending_check(
    defined: &BTreeSet<String>,
    applied: Result<BTreeSet<String>, String>,
) -> Check {
    const ID: &str = "migrations.applied";
    const TITLE: &str = "Applied migrations";
    match applied {
        Ok(applied) => {
            let pending: Vec<&String> = defined.difference(&applied).collect();
            if pending.is_empty() {
                Check::pass(ID, TITLE, format!("all {} applied", defined.len()))
            } else {
                let first = pending
                    .first()
                    .map(|name| display_safe(name))
                    .unwrap_or_default();
                Check::warn(
                    ID,
                    TITLE,
                    format!("{} pending (first: {first})", pending.len()),
                    "cargo rullst db:migrate",
                )
            }
        }
        Err(error) if error.contains("no such table") => Check::warn(
            ID,
            TITLE,
            "no migration has been applied yet",
            "cargo rullst db:migrate",
        ),
        Err(error) => Check::warn(
            ID,
            TITLE,
            format!(
                "could not read the migrations table: {}",
                crate::ui::error_report::sanitize(error.lines().next().unwrap_or_default())
            ),
            "cargo rullst db:status",
        ),
    }
}

pub(crate) fn migration_checks(project: &ProjectContext, vars: &impl Vars) -> Vec<Check> {
    let Some(defined) = migration_names(&project.root) else {
        return vec![Check::info(
            "migrations.files",
            "Migration files",
            "no src/migrations directory",
        )];
    };
    if defined.is_empty() {
        return vec![Check::info(
            "migrations.files",
            "Migration files",
            "none defined yet",
        )];
    }
    let latest = defined
        .iter()
        .next_back()
        .map(|name| display_safe(name))
        .unwrap_or_default();
    let mut checks = vec![Check::pass(
        "migrations.files",
        "Migration files",
        format!("{} defined · latest {latest}", defined.len()),
    )];
    let sqlite_file = primary_url(project, vars).and_then(|(url, _)| match parse_target(&url) {
        Target::SqliteFile(path) => Some(resolve(&project.root, &path)),
        _ => None,
    });
    checks.push(match sqlite_file {
        Some(path) if path.is_file() => pending_check(&defined, applied_sqlite(&path)),
        Some(_) => Check::warn(
            "migrations.applied",
            "Applied migrations",
            "the SQLite database does not exist yet",
            "cargo rullst db:migrate",
        ),
        None => Check::info(
            "migrations.applied",
            "Applied migrations",
            "compare with the database using cargo rullst db:status",
        ),
    });
    checks
}

pub(crate) fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

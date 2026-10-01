//! DSN inspection shared by the `Orm::init*` entrypoints.

/// Classifies the connected backend by the DSN scheme, case-insensitively
/// as URL schemes are. `postgresql` is an alias of `postgres`, and SQLx
/// registers `mariadb` as an alias of its MySQL driver. Any other scheme
/// (or a bare SQLite path under `strict-sqlite`) keeps the SQLite dialect;
/// under SQLx `Any` an unknown scheme already fails to connect.
pub(super) fn driver_for_url(database_url: &str) -> &'static str {
    let scheme = database_url
        .split_once(':')
        .map_or("", |(scheme, _)| scheme);
    if scheme.eq_ignore_ascii_case("postgres") || scheme.eq_ignore_ascii_case("postgresql") {
        "postgres"
    } else if scheme.eq_ignore_ascii_case("mysql") || scheme.eq_ignore_ascii_case("mariadb") {
        "mysql"
    } else {
        "sqlite"
    }
}

/// Reports a DSN that still contains a template placeholder such as
/// `[your-database-id]` or `[YOUR-PASSWORD]`. A bracketed IPv6 literal
/// host (RFC 3986, optionally with an RFC 6874 `%25` zone identifier) is
/// not a placeholder.
pub(super) fn has_placeholder_brackets(database_url: &str) -> bool {
    let mut rest = database_url;
    while let Some(open) = rest.find('[') {
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find(']') else {
            return false;
        };
        let literal = &after_open[..close];
        let address = literal
            .split_once('%')
            .map_or(literal, |(address, _)| address);
        if address.parse::<std::net::Ipv6Addr>().is_err() {
            return true;
        }
        rest = &after_open[close + 1..];
    }
    false
}

/// Rejects unconfigured placeholder URLs before they reach the driver
/// (e.g. the Turso template `libsql://[your-database-id].turso.io`, whose
/// brackets URL parsers read as an IPv6 literal).
pub(super) fn ensure_configured_dsn(database_url: &str) -> Result<(), crate::Error> {
    if has_placeholder_brackets(database_url) {
        return Err(crate::Error::Internal(
            "DATABASE_URL contains placeholder brackets like [your-database-id]. \
            Update your .env file with a real connection string."
                .to_string(),
        ));
    }
    Ok(())
}

/// Path and query string of a `sqlite:` DSN, or `None` for another scheme.
fn sqlite_parts(database_url: &str) -> Option<(&str, &str)> {
    if !database_url.starts_with("sqlite") {
        return None;
    }
    let rest = database_url
        .trim_start_matches("sqlite:")
        .trim_start_matches("//")
        .trim_start_matches("file:");
    Some(rest.split_once('?').unwrap_or((rest, "")))
}

/// The `mode` query parameter of a SQLite DSN, if any.
fn sqlite_mode(query: &str) -> Option<&str> {
    query.split('&').find_map(|parameter| {
        parameter
            .split_once('=')
            .filter(|(key, _)| key.eq_ignore_ascii_case("mode"))
            .map(|(_, mode)| mode)
    })
}

/// Reports a SQLite DSN naming an in-memory database (`:memory:` or
/// `mode=memory`), which exists only while one of its connections is open.
pub(super) fn is_sqlite_memory(database_url: &str) -> bool {
    sqlite_parts(database_url).is_some_and(|(path, query)| {
        path == ":memory:"
            || sqlite_mode(query).is_some_and(|mode| mode.eq_ignore_ascii_case("memory"))
    })
}

/// Touch-creates a missing SQLite database file and its directory, so drivers
/// without implicit `mode=rwc` support never hit `SQLITE_CANTOPEN`.
///
/// This happens only when the DSN permits creation: without a `mode`
/// parameter or with `mode=rwc`. An explicit `mode=ro`/`mode=rw` asks SQLite
/// to open an existing database, so a wrong or unmounted path stays an open
/// error instead of silently becoming a new empty database.
pub(super) fn prepare_sqlite_file(database_url: &str) {
    let Some((path_part, query)) = sqlite_parts(database_url) else {
        return;
    };
    if path_part.is_empty()
        || is_sqlite_memory(database_url)
        || sqlite_mode(query).is_some_and(|mode| !mode.eq_ignore_ascii_case("rwc"))
    {
        return;
    }
    let path = std::path::Path::new(path_part);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        let _ = std::fs::create_dir_all(parent);
    }
    if !path.exists() {
        let _ = std::fs::File::create(path);
    }
}

#[cfg(test)]
mod tests {
    use super::{driver_for_url, ensure_configured_dsn};

    #[test]
    fn driver_follows_the_dsn_scheme_and_its_aliases() {
        for (dsn, driver) in [
            ("postgres://app:pw@db/app", "postgres"),
            ("postgresql://app:pw@db/app", "postgres"),
            ("POSTGRES://app:pw@db/app", "postgres"),
            ("mysql://app:pw@db/app", "mysql"),
            ("mariadb://app:pw@db/app", "mysql"),
            ("MariaDB://app:pw@db/app", "mysql"),
            ("sqlite::memory:", "sqlite"),
            ("sqlite://data/app.db", "sqlite"),
            ("data/app.db", "sqlite"),
        ] {
            assert_eq!(driver_for_url(dsn), driver, "{dsn}");
        }
    }

    #[test]
    fn ipv6_literal_hosts_are_not_template_placeholders() {
        for dsn in [
            "postgres://app:pw@[2001:db8::10]:5432/app",
            "mysql://app:pw@[::1]/app",
            "postgres://app@[fe80::1%25eth0]:5432/app",
        ] {
            assert!(ensure_configured_dsn(dsn).is_ok(), "{dsn}");
        }
        for dsn in [
            "libsql://[your-database-id].turso.io",
            "postgres://app:[YOUR-PASSWORD]@db.example.com/app",
            "postgres://app@[::1]:5432/[database]",
        ] {
            assert!(
                matches!(ensure_configured_dsn(dsn), Err(crate::Error::Internal(_))),
                "{dsn}"
            );
        }
    }
}

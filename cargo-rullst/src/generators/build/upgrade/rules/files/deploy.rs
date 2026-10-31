//! Deployment files written before 13.0.0-alpha.4 changed what `cargo rullst
//! deploy` and `dockerize` generate: the SQLite `DATABASE_URL` of the
//! Dockerfile and the health checks of the Caddyfile.

use super::{line_of, read_small};
use crate::generators::project::database_provider;
use std::path::Path;

/// The runtime stage (the last `FROM` line) of the root `Dockerfile` of a
/// SQLite project when it sets no `DATABASE_URL`, which `deploy` warns about.
/// The database is the one `deploy` reads (`Rullst.toml` `[database] url`,
/// else `.env.example` `DATABASE_URL`), from regular bounded files only.
pub(super) fn dockerfile_without_database_url(root: &Path) -> Option<usize> {
    let dockerfile = read_small(&root.join("Dockerfile"))?;
    let sqlite = database_provider(
        read_small(&root.join("Rullst.toml")).as_deref(),
        read_small(&root.join(".env.example")).as_deref(),
    ) == Some("Sqlite");
    if !sqlite || dockerfile.contains("DATABASE_URL") {
        return None;
    }
    let runtime = dockerfile
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            line.trim_start()
                .get(..5)
                .is_some_and(|word| word.eq_ignore_ascii_case("FROM "))
        })
        .last();
    Some(runtime.map_or(1, |(index, _)| index + 1))
}

/// The `reverse_proxy app:3000` line of a `Caddyfile` that lacks the
/// `health_uri` or `lb_try_duration` directive `deploy` writes since
/// 13.0.0-alpha.4: without them Caddy keeps routing to an app that is
/// starting or restarting and answers with an error at once.
pub(super) fn caddyfile_without_health_checks(text: &str) -> Option<usize> {
    let has = |directive: &str| line_of(text, |line| line.starts_with(directive)).is_some();
    if has("health_uri") && has("lb_try_duration") {
        return None;
    }
    line_of(text, |line| line.starts_with("reverse_proxy app:3000"))
}

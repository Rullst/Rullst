//! Fast-linker hint (Linux only). Linking is a large part of every rebuild of
//! a Rullst binary; `mold` or LLD can shorten it. The check is advisory: a
//! `!` at most, never a failure. It appears only on Linux when no Cargo
//! configuration or environment variable selects a linker and rustc does not
//! already link the host with its bundled LLD (the default on
//! x86_64-unknown-linux-gnu since Rust 1.90).

use super::context::Vars;
use super::probe::{self, Probe};
use super::report::Check;
use super::toolchain::{RustVersion, parse_rustc_version};
use crate::ui::home::read_small_file;
use std::path::{Path, PathBuf};

const ID: &str = "toolchain.linker";
const TITLE: &str = "Fast linker";
const CONFIG_LIMIT: u64 = 1024 * 1024;
/// The release that made `rust-lld` the x86_64-unknown-linux-gnu default.
const RUST_LLD_DEFAULT_SINCE: RustVersion = RustVersion(1, 90, 0);

/// What the doctor knows about linking on this host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkerFacts {
    /// The `[target...]` table the suggested snippet uses.
    pub table: String,
    /// A Cargo configuration or environment variable already selects one.
    pub configured: bool,
    /// rustc already links this host with LLD by default.
    pub default_lld: bool,
    pub mold: bool,
    pub lld: bool,
}

/// The host's target triple for the common Linux hosts, if known.
fn host_triple() -> Option<String> {
    let env = if cfg!(target_env = "musl") {
        "musl"
    } else {
        "gnu"
    };
    matches!(std::env::consts::ARCH, "x86_64" | "aarch64")
        .then(|| format!("{}-unknown-linux-{env}", std::env::consts::ARCH))
}

/// Facts for the Linux host, or `None` on other operating systems.
pub(crate) fn detect(cwd: &Path, rustc: &Probe, vars: &impl Vars) -> Option<LinkerFacts> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let triple = host_triple();
    let x86_64_gnu = triple.as_deref() == Some("x86_64-unknown-linux-gnu");
    let version = rustc
        .ok()
        .then(|| parse_rustc_version(&rustc.first_line))
        .flatten();
    let cargo_home = vars
        .var("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| vars.var("HOME").map(|home| Path::new(&home).join(".cargo")));
    Some(LinkerFacts {
        table: triple.as_deref().map_or_else(
            || "target.'cfg(target_os = \"linux\")'".to_string(),
            |triple| format!("target.{triple}"),
        ),
        configured: configured(cwd, cargo_home.as_deref(), triple.as_deref(), vars),
        // An unknown rustc already fails `toolchain.rustc`; no extra hint then.
        default_lld: x86_64_gnu && version.is_none_or(|found| found >= RUST_LLD_DEFAULT_SINCE),
        mold: probe::installed("mold").found,
        lld: probe::installed("ld.lld").found,
    })
}

/// Cargo's configuration files for `cwd` (it and every ancestor), then
/// Cargo's home, in Cargo's lookup order.
pub(crate) fn config_files(cwd: &Path, cargo_home: Option<&Path>) -> Vec<PathBuf> {
    let mut directories: Vec<PathBuf> = cwd.ancestors().map(|dir| dir.join(".cargo")).collect();
    directories.extend(cargo_home.map(Path::to_path_buf));
    directories
        .into_iter()
        .flat_map(|dir| [dir.join("config.toml"), dir.join("config")])
        .collect()
}

/// Whether a configuration file or environment variable selects a linker.
pub(crate) fn configured(
    cwd: &Path,
    cargo_home: Option<&Path>,
    triple: Option<&str>,
    vars: &impl Vars,
) -> bool {
    let target_var = triple.map(|triple| triple.replace('-', "_").to_ascii_uppercase());
    let flag_vars = [
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
    ]
    .into_iter()
    .map(str::to_string)
    .chain(
        target_var
            .iter()
            .map(|t| format!("CARGO_TARGET_{t}_RUSTFLAGS")),
    );
    let environment = flag_vars
        .filter_map(|name| vars.var(&name))
        .any(|flags| selects_fast_linker(&flags))
        || target_var
            .as_deref()
            .is_some_and(|t| vars.var(&format!("CARGO_TARGET_{t}_LINKER")).is_some());
    environment
        || config_files(cwd, cargo_home).iter().any(|path| {
            read_small_file(path, CONFIG_LIMIT)
                .ok()
                .flatten()
                .is_some_and(|contents| config_selects_linker(&contents, triple))
        })
}

/// Whether one `.cargo/config.toml` selects a linker for the host: a
/// `linker` entry, or `-fuse-ld=mold`/`-fuse-ld=lld` in the rustflags of
/// `[build]`, of the host's `[target.<triple>]` or of a `[target.'cfg(..)']`.
pub(crate) fn config_selects_linker(contents: &str, triple: Option<&str>) -> bool {
    let Ok(config) = toml::from_str::<toml::Value>(contents) else {
        return false;
    };
    let flags_select = |table: &toml::Value| match table.get("rustflags") {
        Some(toml::Value::String(flags)) => selects_fast_linker(flags),
        Some(toml::Value::Array(flags)) => flags
            .iter()
            .filter_map(toml::Value::as_str)
            .any(selects_fast_linker),
        _ => false,
    };
    if config.get("build").is_some_and(flags_select) {
        return true;
    }
    config
        .get("target")
        .and_then(toml::Value::as_table)
        .is_some_and(|targets| {
            targets
                .iter()
                .filter(|(key, _)| Some(key.as_str()) == triple || key.starts_with("cfg("))
                .any(|(_, table)| table.get("linker").is_some() || flags_select(table))
        })
}

fn selects_fast_linker(flags: &str) -> bool {
    ["-fuse-ld=", "--ld-path="].iter().any(|option| {
        flags.match_indices(option).any(|(start, _)| {
            let linker = flags[start + option.len()..]
                .split(|c: char| c.is_whitespace() || c == '\u{1f}' || c == '"')
                .next()
                .unwrap_or_default();
            linker.contains("mold") || linker.contains("lld")
        })
    })
}

/// The `!` hint, or `None` when a fast linker is configured or the default.
pub(crate) fn check(facts: &LinkerFacts) -> Option<Check> {
    if facts.configured || facts.default_lld {
        return None;
    }
    let (detail, linker) = match (facts.mold, facts.lld) {
        (true, _) => ("not configured; mold is installed", "mold"),
        (false, true) => ("not configured; ld.lld is installed", "lld"),
        (false, false) => (
            "not configured; neither mold nor ld.lld is installed",
            "mold",
        ),
    };
    let start = if facts.mold || facts.lld {
        "Add"
    } else {
        "Install mold with your package manager, then add"
    };
    let fix = format!(
        "{start} to .cargo/config.toml:\n[{}]\nrustflags = [\"-C\", \"link-arg=-fuse-ld={linker}\"]",
        facts.table
    );
    Some(Check::warn(ID, TITLE, detail, fix))
}

#[cfg(test)]
#[path = "linker_tests.rs"]
mod tests;

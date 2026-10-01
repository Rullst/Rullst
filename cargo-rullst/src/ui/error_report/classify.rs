//! Maps the most common CLI failures to a short title, a fix and a docs link.
//! [`Facts`] is extracted once from the error chain, so the mapping itself is
//! a pure, unit-tested function.

use regex::Regex;
use std::error::Error;
use std::io::ErrorKind;
use std::sync::OnceLock;

pub(super) const START_HERE: &str = "https://rullst.github.io/Rullst/book/start-here.html";
pub(super) const ERRORS_DOCS: &str =
    "https://rullst.github.io/Rullst/book/cli_reference.html#errors-and-exit-codes";
const DOCTOR_DOCS: &str =
    "https://rullst.github.io/Rullst/book/cli_reference.html#cargo-rullst-doctor";
const DEV_DOCS: &str = "https://rullst.github.io/Rullst/book/cli_reference.html#cargo-rullst-dev";
const DATABASE_DOCS: &str =
    "https://rullst.github.io/Rullst/book/cli_reference.html#cargo-rullst-dbmigrate";

/// Everything the mapping needs, gathered from an error chain.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Facts {
    /// The top-level message, unredacted (rendering redacts it).
    pub message: String,
    /// Every message of the chain, lowercased and joined by newlines.
    pub text: String,
    pub io_kinds: Vec<ErrorKind>,
    pub project_required: bool,
    /// A database driver reported a connection-level failure.
    pub database: bool,
    /// An HTTP client reported a connection, timeout or request failure.
    pub network: bool,
    /// The subcommand that failed, such as `make:model`.
    pub command: Option<String>,
    /// `..`-style path to an enclosing Rullst project, when there is one.
    pub project_root: Option<String>,
}

impl Facts {
    pub(crate) fn from_error(
        error: &(dyn Error + 'static),
        command: Option<String>,
        project_root: Option<String>,
    ) -> Self {
        let mut facts = Self {
            message: error.to_string(),
            command,
            project_root,
            ..Self::default()
        };
        let mut texts = Vec::new();
        let mut current: Option<&(dyn Error + 'static)> = Some(error);
        let mut depth = 0;
        while let Some(error) = current {
            texts.push(error.to_string().to_lowercase());
            if let Some(io) = error.downcast_ref::<std::io::Error>() {
                facts.io_kinds.push(io.kind());
            }
            if let Some(dialoguer::Error::IO(io)) = error.downcast_ref::<dialoguer::Error>() {
                facts.io_kinds.push(io.kind());
            }
            if error.downcast_ref::<super::ProjectRequired>().is_some() {
                facts.project_required = true;
            }
            if let Some(database) = error.downcast_ref::<sqlx::Error>() {
                facts.database |= matches!(
                    database,
                    sqlx::Error::Io(_)
                        | sqlx::Error::Tls(_)
                        | sqlx::Error::PoolTimedOut
                        | sqlx::Error::PoolClosed
                );
            }
            if let Some(http) = error.downcast_ref::<reqwest::Error>() {
                facts.network |= http.is_connect() || http.is_timeout() || http.is_request();
            }
            depth += 1;
            current = if depth < 32 { error.source() } else { None };
        }
        facts.text = texts.join("\n");
        facts
    }

    fn has(&self, kind: ErrorKind) -> bool {
        self.io_kinds.contains(&kind)
    }

    fn mentions(&self, needles: &[&str]) -> bool {
        needles.iter().any(|needle| self.text.contains(needle))
    }

    fn database_command(&self) -> bool {
        self.command.as_deref().is_some_and(|command| {
            command.starts_with("db:")
                || matches!(
                    command,
                    "studio" | "generate:models" | "make:models-from-db" | "make:migration:auto"
                )
        })
    }
}

/// The rendered parts of a friendly error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Friendly {
    pub title: String,
    pub happened: String,
    pub fix: Option<String>,
    pub docs: Option<&'static str>,
}

impl Friendly {
    fn new(title: impl Into<String>, facts: &Facts) -> Self {
        Self {
            title: title.into(),
            happened: facts.message.clone(),
            fix: None,
            docs: None,
        }
    }

    fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }

    fn docs(mut self, docs: &'static str) -> Self {
        self.docs = Some(docs);
        self
    }
}

fn regex(cell: &'static OnceLock<Option<Regex>>, pattern: &str) -> Option<&'static Regex> {
    cell.get_or_init(|| Regex::new(pattern).ok()).as_ref()
}

/// Tools whose absence is common enough to deserve an install hint.
const TOOLS: [(&str, &str); 15] = [
    (
        "rustup",
        "Install Rust with rustup from https://rustup.rs, then open a new shell.",
    ),
    (
        "rustc",
        "Install Rust with rustup from https://rustup.rs, then open a new shell.",
    ),
    (
        "cargo",
        "Install Rust with rustup from https://rustup.rs, then open a new shell.",
    ),
    ("git", "Install Git from https://git-scm.com/downloads."),
    (
        "docker",
        "Install Docker from https://docs.docker.com/get-docker/ and start it.",
    ),
    (
        "flyctl",
        "Install the Fly.io CLI from https://fly.io/docs/flyctl/install/.",
    ),
    (
        "railway",
        "Install the Railway CLI from https://docs.railway.com/guides/cli.",
    ),
    ("wasm-bindgen", "Run `cargo install wasm-bindgen-cli`."),
    (
        "kubectl",
        "Install kubectl from https://kubernetes.io/docs/tasks/tools/.",
    ),
    (
        "buildah",
        "Install Buildah with your system package manager.",
    ),
    ("nix", "Install Nix from https://nixos.org/download/."),
    (
        "protoc",
        "Install the Protobuf compiler (protoc) with your package manager.",
    ),
    ("npm", "Install Node.js (npm) from https://nodejs.org/."),
    (
        "ssh",
        "Install an OpenSSH client with your system package manager.",
    ),
    (
        "tauri",
        "Run `cargo install tauri-cli` or install it with npm.",
    ),
];

/// The missing tool named by a "not found"-style message.
fn missing_tool(text: &str) -> Option<(&'static str, &'static str)> {
    static AFTER: OnceLock<Option<Regex>> = OnceLock::new();
    static BEFORE: OnceLock<Option<Regex>> = OnceLock::new();
    let names = "rustup|rustc|cargo|git|docker|flyctl|railway|wasm-bindgen|kubectl|buildah|nix|protoc|npm|ssh|tauri";
    let after = regex(
        &AFTER,
        &format!(
            r#"(?:^|[\s'"`(])({names})['"`)]?(?::\s*command)?\s+(?:was\s+)?(?:not found|is not installed|not installed|is unavailable|is not available|could not be found)"#
        ),
    )?;
    let before = regex(
        &BEFORE,
        &format!(
            r#"(?:failed to (?:run|execute|spawn|start)|could not (?:run|execute|find|start)|unable to (?:run|execute|find|start)|install)\s+['"`]?({names})\b"#
        ),
    )?;
    let name = after
        .captures(text)
        .or_else(|| before.captures(text))?
        .get(1)?
        .as_str();
    TOOLS.iter().copied().find(|(tool, _)| *tool == name)
}

/// The Rust target named by a missing-target compiler message.
fn missing_target(text: &str) -> Option<String> {
    static TARGET: OnceLock<Option<Regex>> = OnceLock::new();
    if !(text.contains("target may not be installed")
        || text.contains("can't find crate for `std`")
        || text.contains("can't find crate for `core`")
        || text.contains("rustup target add"))
    {
        return None;
    }
    let target = regex(
        &TARGET,
        r"\b((?:wasm32|wasm64|x86_64|aarch64|arm|armv7|i686|riscv64gc|thumbv[0-9a-z]+)-[a-z0-9_]+(?:-[a-z0-9_]+){0,2})\b",
    )?;
    Some(
        target
            .captures(text)
            .and_then(|captures| captures.get(1))
            .map_or_else(|| "<target>".to_string(), |m| m.as_str().to_string()),
    )
}

fn port(text: &str) -> Option<String> {
    static PORT: OnceLock<Option<Regex>> = OnceLock::new();
    regex(&PORT, r"(?:port\s+|:)(\d{2,5})\b")?
        .captures(text)
        .and_then(|captures| captures.get(1))
        .map(|m| m.as_str().to_string())
}

fn not_in_project(facts: &Facts) -> bool {
    facts.project_required
        || facts.text.lines().any(|line| {
            line.contains("rullst project")
                && (line.contains("must") || line.contains("run ") || line.contains("are you in"))
        })
}

fn port_in_use(facts: &Facts) -> bool {
    facts.has(ErrorKind::AddrInUse)
        || facts.mentions(&[
            "address already in use",
            "address in use",
            "os error 98)",
            "os error 48)",
            "os error 10048)",
        ])
}

fn permission_denied(facts: &Facts) -> bool {
    facts.has(ErrorKind::PermissionDenied)
        || facts.mentions(&["permission denied", "os error 13)", "access is denied"])
}

fn database_unreachable(facts: &Facts) -> bool {
    facts.database
        || facts.mentions(&[
            "error communicating with database",
            "pool timed out while waiting for an open connection",
        ])
        || ((facts.has(ErrorKind::ConnectionRefused) || facts.mentions(&["connection refused"]))
            && (facts.database_command()
                || facts.mentions(&["database", "postgres", "mysql", "mariadb"])))
}

fn network_failure(facts: &Facts) -> bool {
    facts.network
        || facts.has(ErrorKind::ConnectionRefused)
        || facts.has(ErrorKind::TimedOut)
        || facts.has(ErrorKind::ConnectionReset)
        || facts.mentions(&[
            "dns error",
            "failed to lookup address",
            "network is unreachable",
            "error sending request",
            "connection refused",
            "connection reset",
            "operation timed out",
            "connection timed out",
        ])
}

/// The friendly form of `facts`; a generic message when nothing matches.
pub(crate) fn classify(facts: &Facts) -> Friendly {
    if facts.has(ErrorKind::Interrupted) || facts.text.contains("read interrupted") {
        return Friendly::new("Cancelled", facts);
    }
    if not_in_project(facts) {
        let mut friendly = Friendly::new("Not inside a Rullst project", facts).docs(START_HERE);
        if facts.project_required {
            let command = facts.command.as_deref().map_or_else(
                || "This command".to_string(),
                |name| format!("`cargo rullst {name}`"),
            );
            friendly.happened = format!(
                "{command} must run at the root of a Rullst project; this directory has no \
                 Cargo.toml with a `rullst` dependency."
            );
        }
        return match &facts.project_root {
            Some(root) => friendly.fix(format!(
                "The project root is `{root}`: run `cd {root}` and try again."
            )),
            None => friendly.fix(
                "Run the command from your project root (the folder whose Cargo.toml depends on \
                 `rullst`), or create a project with `cargo rullst new <name>`.",
            ),
        };
    }
    if port_in_use(facts) {
        let fix = match port(&facts.text) {
            Some(port) => format!(
                "Stop the process listening on port {port} ({}) or set PORT in .env to a free port.",
                port_lookup(&port)
            ),
            None => "Stop the other server or set PORT in .env to a free port.".to_string(),
        };
        return Friendly::new("Port already in use", facts)
            .fix(fix)
            .docs(DEV_DOCS);
    }
    if permission_denied(facts) {
        return Friendly::new("Permission denied", facts)
            .fix(
                "Check the owner and permissions of the files involved; avoid running cargo \
                 rullst with sudo inside a project you own.",
            )
            .docs(ERRORS_DOCS);
    }
    if let Some(target) = missing_target(&facts.text) {
        return Friendly::new("Rust target not installed", facts)
            .fix(format!("Run `rustup target add {target}` and try again."))
            .docs(DOCTOR_DOCS);
    }
    if let Some((tool, install)) = missing_tool(&facts.text) {
        return Friendly::new(format!("`{tool}` is not installed"), facts)
            .fix(format!(
                "{install} `cargo rullst doctor` shows the rest of the toolchain."
            ))
            .docs(DOCTOR_DOCS);
    }
    if database_unreachable(facts) {
        return Friendly::new("Database unreachable", facts)
            .fix(
                "Start the database server (for example `docker compose up -d`) or correct \
                 DATABASE_URL in .env; `cargo rullst doctor` checks the connection.",
            )
            .docs(DATABASE_DOCS);
    }
    if network_failure(facts) {
        return Friendly::new("Network request failed", facts)
            .fix(
                "Check your internet connection and proxy settings (HTTPS_PROXY, NO_PROXY), \
                 then try again.",
            )
            .docs(ERRORS_DOCS);
    }
    if facts.has(ErrorKind::InvalidInput) {
        let help = match &facts.command {
            Some(command) => format!("`cargo rullst {command} --help`"),
            None => "`cargo rullst --help`".to_string(),
        };
        return Friendly::new("Invalid input", facts)
            .fix(format!(
                "Check the arguments and values; {help} lists them."
            ))
            .docs(ERRORS_DOCS);
    }
    if facts.has(ErrorKind::NotFound) {
        return Friendly::new("File or program not found", facts)
            .fix("Check the path in the message; `cargo rullst doctor` lists missing tools.")
            .docs(DOCTOR_DOCS);
    }
    let title = match &facts.command {
        Some(command) => format!("`cargo rullst {command}` failed"),
        None => "Command failed".to_string(),
    };
    Friendly::new(title, facts)
        .fix("Run it again with -v for the underlying causes; `cargo rullst doctor` checks your setup.")
        .docs(ERRORS_DOCS)
}

#[cfg(windows)]
fn port_lookup(port: &str) -> String {
    format!("`netstat -ano | findstr :{port}` shows it")
}

#[cfg(not(windows))]
fn port_lookup(port: &str) -> String {
    format!("`lsof -i :{port}` shows it")
}

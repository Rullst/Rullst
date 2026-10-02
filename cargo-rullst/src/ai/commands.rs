//! The only processes the assistant may start: an allowlisted `cargo rullst`
//! subcommand, `cargo check` or `cargo test`. Arguments pass a strict token
//! grammar; [`super::process`] runs them without a shell.

use std::time::Duration;

/// `cargo rullst` subcommands the assistant may propose inside a project.
/// Deliberately absent: deploy, foundry:*, upgrade, update, pkg, dev, dash,
/// studio, build*, omni, eject, hook:install, db:rollback/seed, auth and
/// generate:models (which would route a database connection string through
/// the model). `new` is validated separately by [`validate_new`], and
/// `db:migrate` is further limited to development projects by the caller.
pub(super) const RULLST_ALLOWLIST: &[&str] = &[
    "make:controller",
    "make:model",
    "make:resource",
    "make:middleware",
    "make:migration",
    "make:migration:auto",
    "make:billing",
    "make:omni",
    "make:iot",
    "make:mail",
    "make:mail-invoice",
    "make:mail-dunning",
    "make:cors",
    "make:jwt",
    "make:worker",
    "make:island",
    "make:chat-session",
    "make:k8s",
    "make:mfa",
    "make:scalar",
    "make:live",
    "make:grpc",
    "make:privacy",
    "make:age-gate",
    "generate:openapi",
    "generate:ts",
    "generate:diagram",
    "generate:ai-context",
    "generate:api",
    "generate:buildah",
    "db:status",
    "db:migrate",
    "doctor",
    "audit",
    "inspect",
];

/// Flags refused even on allowlisted commands: `doctor --fix` installs
/// software and `audit --network` scans local listeners.
const FORBIDDEN_FLAGS: &[(&str, &str)] = &[("doctor", "--fix"), ("audit", "--network")];

/// The only `inspect` targets: any other target is read as a file path and
/// printed, which would bypass the path policy (`.env`, keys, symlinks).
const INSPECT_TARGETS: &[&str] = &["routes", "route", "models", "model", "schema"];

const CARGO_CHECK_FLAGS: &[&str] = &[
    "--all-targets",
    "--tests",
    "--lib",
    "--bins",
    "--all-features",
    "--quiet",
    "-q",
    "--message-format=short",
];
const CARGO_TEST_FLAGS: &[&str] = &[
    "--lib",
    "--tests",
    "--all-targets",
    "--all-features",
    "--no-fail-fast",
    "--quiet",
    "-q",
];
const MAX_ARGS: usize = 8;
const MAX_ARG_BYTES: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum CommandKind {
    /// `cargo rullst <args>`
    Rullst,
    /// `cargo check|test <args>`
    Cargo,
}

/// A validated invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Invocation {
    pub kind: CommandKind,
    pub args: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum CommandError {
    #[error("`{0}` is not an allowlisted command")]
    NotAllowed(String),
    #[error("argument `{0}` is not allowed")]
    Argument(String),
    #[error("a command needs a subcommand and at most 8 arguments")]
    Arity,
}

/// One argument token: no whitespace, quotes, shell metacharacters, `..`,
/// absolute paths or home-relative paths. Programs are not run through a
/// shell; the grammar keeps the displayed command unambiguous.
fn safe_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ARG_BYTES
        && !value.contains("..")
        && !value.starts_with('/')
        && !value.starts_with('~')
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'_' | b'-' | b'.' | b'/' | b':' | b'=' | b',' | b'@' | b'+'
                )
        })
}

/// A flag (`-m`, `--api`, `--name=value`) or a positional value.
fn safe_argument(value: &str) -> bool {
    if !safe_token(value) {
        return false;
    }
    match value.strip_prefix("--").or_else(|| value.strip_prefix('-')) {
        Some(flag) => {
            let name = flag.split_once('=').map_or(flag, |(name, _)| name);
            name.starts_with(|c: char| c.is_ascii_alphabetic())
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        }
        None => true,
    }
}

fn arity(args: &[String]) -> Result<(), CommandError> {
    if args.is_empty() || args.len() > MAX_ARGS + 1 {
        Err(CommandError::Arity)
    } else {
        Ok(())
    }
}

/// Validates `cargo rullst <args>`.
pub(super) fn validate_rullst(args: Vec<String>) -> Result<Invocation, CommandError> {
    arity(&args)?;
    let subcommand = args[0].as_str();
    if !RULLST_ALLOWLIST.contains(&subcommand) {
        return Err(CommandError::NotAllowed(format!(
            "cargo rullst {}",
            truncate(subcommand)
        )));
    }
    if subcommand == "inspect"
        && let Some(refused) = args
            .get(1)
            .filter(|target| !INSPECT_TARGETS.contains(&target.as_str()))
            .or_else(|| args.get(2))
    {
        return Err(CommandError::Argument(truncate(refused)));
    }
    for argument in &args[1..] {
        let flag = argument
            .split_once('=')
            .map_or(argument.as_str(), |(flag, _)| flag);
        if !safe_argument(argument)
            || FORBIDDEN_FLAGS
                .iter()
                .any(|(command, forbidden)| *command == subcommand && *forbidden == flag)
        {
            return Err(CommandError::Argument(truncate(argument)));
        }
    }
    Ok(Invocation {
        kind: CommandKind::Rullst,
        args,
    })
}

const NEW_FLAGS: &[&str] = &[
    "--default",
    "--api",
    "--ai",
    "--redis",
    "--no-database",
    "--skip-initial-migration",
];
const BLUEPRINTS: &[&str] = &["blank", "lms", "saas", "blog", "portfolio", "erp"];
const DATABASES: &[&str] = &["sqlite", "postgres", "mysql", "mariadb", "turso"];

/// A project name the CLI accepts: a lowercase package name.
pub(super) fn valid_project_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|first| first.is_ascii_lowercase())
        && name.len() <= 64
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

/// Validates `cargo rullst new <name> [flags]`, used only outside a project.
/// `--default` is added when missing so generation never prompts; blueprint
/// and database values must be ones the CLI accepts.
pub(super) fn validate_new(args: Vec<String>) -> Result<(Invocation, String), CommandError> {
    if args.len() < 2 || args.len() > MAX_ARGS + 1 || args[0] != "new" {
        return Err(CommandError::Arity);
    }
    let name = args[1].clone();
    if !valid_project_name(&name) {
        return Err(CommandError::Argument(truncate(&name)));
    }
    let mut checked = vec!["new".to_string(), name.clone()];
    let mut iter = args[2..].iter();
    while let Some(argument) = iter.next() {
        let (flag, inline) = match argument.split_once('=') {
            Some((flag, value)) => (flag, Some(value.to_string())),
            None => (argument.as_str(), None),
        };
        let values = match flag {
            "--blueprint" => BLUEPRINTS,
            "--database" => DATABASES,
            flag if NEW_FLAGS.contains(&flag) && inline.is_none() => {
                checked.push(flag.to_string());
                continue;
            }
            _ => return Err(CommandError::Argument(truncate(argument))),
        };
        let value = match inline {
            Some(value) => value,
            None => iter
                .next()
                .cloned()
                .ok_or_else(|| CommandError::Argument(flag.to_string()))?,
        };
        if !values.contains(&value.as_str()) {
            return Err(CommandError::Argument(truncate(&value)));
        }
        checked.push(flag.to_string());
        checked.push(value);
    }
    if !checked.iter().any(|argument| argument == "--default") {
        checked.insert(2, "--default".to_string());
    }
    Ok((
        Invocation {
            kind: CommandKind::Rullst,
            args: checked,
        },
        name,
    ))
}

/// Validates `cargo check|test <args>`. `test` accepts one test-name filter
/// and `--test <name>`; nothing may redirect the manifest, target directory,
/// configuration or toolchain.
pub(super) fn validate_cargo(args: Vec<String>) -> Result<Invocation, CommandError> {
    arity(&args)?;
    let (allowed, test) = match args[0].as_str() {
        "check" => (CARGO_CHECK_FLAGS, false),
        "test" => (CARGO_TEST_FLAGS, true),
        other => {
            return Err(CommandError::NotAllowed(format!(
                "cargo {}",
                truncate(other)
            )));
        }
    };
    let identifier = |value: &str| {
        !value.is_empty()
            && value.len() <= MAX_ARG_BYTES
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
    };
    let mut filter_seen = false;
    let mut iter = args[1..].iter();
    while let Some(argument) = iter.next() {
        if allowed.contains(&argument.as_str()) {
            continue;
        }
        if test && argument == "--test" {
            match iter.next() {
                Some(name) if identifier(name) => continue,
                _ => return Err(CommandError::Argument("--test".to_string())),
            }
        }
        if test && !filter_seen && identifier(argument) {
            filter_seen = true;
            continue;
        }
        return Err(CommandError::Argument(truncate(argument)));
    }
    Ok(Invocation {
        kind: CommandKind::Cargo,
        args,
    })
}

fn truncate(value: &str) -> String {
    let mut output: String = value.chars().take(64).collect();
    if output.len() < value.len() {
        output.push('…');
    }
    super::term::sanitize(&output)
}

impl Invocation {
    /// The exact command line shown to the user.
    pub(super) fn display(&self) -> String {
        let program = match self.kind {
            CommandKind::Rullst => "cargo rullst",
            CommandKind::Cargo => "cargo",
        };
        format!("{program} {}", self.args.join(" "))
    }

    fn subcommand(&self) -> Option<&str> {
        self.args.first().map(String::as_str)
    }

    /// Commands confirmed one by one even after "all": they create a project
    /// or change a database, which no git checkpoint can undo.
    pub(super) fn always_confirm(&self) -> bool {
        self.kind == CommandKind::Rullst && matches!(self.subcommand(), Some("new" | "db:migrate"))
    }

    /// Whether the command may write project files (and so needs a checkpoint).
    pub(super) fn mutates(&self) -> bool {
        match self.kind {
            CommandKind::Cargo => false,
            CommandKind::Rullst => match self.subcommand() {
                Some("db:status" | "doctor" | "inspect" | "new" | "db:migrate") => false,
                Some("audit") => self
                    .args
                    .iter()
                    .any(|arg| arg == "--compliance" || arg == "--sbom"),
                _ => true,
            },
        }
    }

    pub(super) fn deadline(&self) -> Duration {
        match self.kind {
            // Scaffolding a project or migrating may build the application.
            CommandKind::Rullst if self.always_confirm() => Duration::from_secs(30 * 60),
            CommandKind::Rullst => Duration::from_secs(5 * 60),
            CommandKind::Cargo => Duration::from_secs(30 * 60),
        }
    }

    pub(super) fn program(&self) -> std::io::Result<std::path::PathBuf> {
        #[cfg(test)]
        if let Some(program) = TEST_RULLST_PROGRAM.with(|program| program.borrow().clone())
            && self.kind == CommandKind::Rullst
        {
            return Ok(program);
        }
        match self.kind {
            CommandKind::Rullst => std::env::current_exe(),
            // Cargo sets `CARGO` for subcommands; otherwise use the PATH entry.
            CommandKind::Cargo => Ok(std::env::var_os("CARGO")
                .map(std::path::PathBuf::from)
                .filter(|path| path.is_absolute())
                .unwrap_or_else(|| "cargo".into())),
        }
    }
}

#[cfg(test)]
thread_local! {
    /// A stand-in for the CLI binary in unit tests, where the current
    /// executable is the test harness.
    pub(super) static TEST_RULLST_PROGRAM: std::cell::RefCell<Option<std::path::PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
#[path = "tests/commands.rs"]
mod tests;

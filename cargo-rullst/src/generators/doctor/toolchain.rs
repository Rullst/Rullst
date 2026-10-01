//! Toolchain checks: Rust (MSRV), Cargo, rustfmt/clippy, Git, the Wasm
//! target when the project has islands, and the optional analysis tools.

use super::probe::{self, Probe};
use super::report::Check;

pub(crate) const RULLST_MSRV: RustVersion = RustVersion(1, 96, 0);
pub(crate) const WASM_TARGET: &str = "wasm32-unknown-unknown";
pub(crate) const COMPONENT_FIX: &str =
    "rustup component add rustfmt clippy (or cargo rullst doctor --fix)";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RustVersion(pub u32, pub u32, pub u32);

pub(crate) fn parse_rustc_version(output: &str) -> Option<RustVersion> {
    let raw = output.split_whitespace().nth(1)?;
    let core = raw.split_once('-').map_or(raw, |(version, _)| version);
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some(RustVersion(major, minor, patch))
}

/// Optional tools: (display name, probe arguments after the program, install hint).
const OPTIONAL: [(&str, &str, &[&str], &str); 6] = [
    (
        "cargo-deny",
        "cargo",
        &["deny", "--version"],
        "cargo install cargo-deny",
    ),
    (
        "cargo-geiger",
        "cargo",
        &["geiger", "--version"],
        "cargo install cargo-geiger",
    ),
    (
        "cargo-mutants",
        "cargo",
        &["mutants", "--version"],
        "cargo install cargo-mutants",
    ),
    (
        "kani",
        "cargo",
        &["kani", "--version"],
        "cargo install kani-verifier && cargo kani setup",
    ),
    (
        "cargo-llvm-cov",
        "cargo",
        &["llvm-cov", "--version"],
        "cargo install cargo-llvm-cov",
    ),
    (
        "docker",
        "docker",
        &["--version"],
        "https://docs.docker.com/get-docker/",
    ),
];

/// Every program the toolchain and security groups need, probed concurrently.
pub(crate) struct Probes {
    pub rustc: Probe,
    pub cargo: Probe,
    pub rustfmt: Probe,
    pub clippy: Probe,
    pub git: Probe,
    pub cargo_audit: Probe,
    pub optional: Vec<Probe>,
    /// `rustup target list --installed`, only when the project has islands.
    pub targets: Option<Probe>,
}

impl Probes {
    pub(crate) fn collect(needs_wasm: bool) -> Self {
        let mut list: Vec<(&str, Vec<&str>)> = vec![
            ("rustc", vec!["--version"]),
            ("cargo", vec!["--version"]),
            ("cargo", vec!["fmt", "--version"]),
            ("cargo", vec!["clippy", "--version"]),
            ("git", vec!["--version"]),
            ("cargo", vec!["audit", "--version"]),
        ];
        list.extend(
            OPTIONAL
                .iter()
                .map(|(_, program, args, _)| (*program, args.to_vec())),
        );
        if needs_wasm {
            list.push(("rustup", vec!["target", "list", "--installed"]));
        }
        let mut results = probe::run_all(&list).into_iter();
        let mut next = || results.next().unwrap_or_default();
        let rustc = next();
        let cargo = next();
        let rustfmt = next();
        let clippy = next();
        let git = next();
        let cargo_audit = next();
        let optional = OPTIONAL.iter().map(|_| next()).collect();
        let targets = needs_wasm.then(&mut next);
        Self {
            rustc,
            cargo,
            rustfmt,
            clippy,
            git,
            cargo_audit,
            optional,
            targets,
        }
    }
}

pub(crate) fn rustc_check(probe: &Probe) -> Check {
    const ID: &str = "toolchain.rustc";
    const TITLE: &str = "Rust compiler";
    if !probe.found {
        return Check::fail(
            ID,
            TITLE,
            "rustc not found",
            "Install Rust from https://rustup.rs",
        );
    }
    if !probe.success {
        return Check::fail(
            ID,
            TITLE,
            "rustc --version failed",
            "Run 'rustup update' to install a working Rust toolchain.",
        );
    }
    let version = probe.first_line.as_str();
    match parse_rustc_version(version) {
        Some(found) if found >= RULLST_MSRV => {
            Check::pass(ID, TITLE, format!("{version} (MSRV 1.96.0)"))
        }
        Some(_) => Check::fail(
            ID,
            TITLE,
            format!("{version} is older than the MSRV 1.96.0"),
            "Run 'rustup update stable' because Rullst requires Rust 1.96.0 or newer.",
        ),
        None => Check::warn(
            ID,
            TITLE,
            format!("unrecognized version output: {version}"),
            "Verify manually that rustc is version 1.96.0 or newer.",
        ),
    }
}

pub(crate) fn cargo_check(probe: &Probe) -> Check {
    if probe.ok() {
        Check::pass("toolchain.cargo", "Cargo", probe.first_line.clone())
    } else {
        Check::fail(
            "toolchain.cargo",
            "Cargo",
            if probe.found {
                "cargo --version failed"
            } else {
                "cargo not found"
            },
            "Install Rust (with Cargo) from https://rustup.rs",
        )
    }
}

pub(crate) fn components_check(rustfmt: &Probe, clippy: &Probe) -> Check {
    const ID: &str = "toolchain.components";
    const TITLE: &str = "rustfmt & clippy";
    let missing: Vec<&str> = [("rustfmt", rustfmt), ("clippy", clippy)]
        .into_iter()
        .filter(|(_, probe)| !probe.ok())
        .map(|(name, _)| name)
        .collect();
    if missing.is_empty() {
        Check::pass(ID, TITLE, "installed")
    } else {
        Check::warn(
            ID,
            TITLE,
            format!("missing: {}", missing.join(", ")),
            COMPONENT_FIX,
        )
    }
}

pub(crate) fn git_check(probe: &Probe) -> Check {
    if probe.ok() {
        Check::pass("toolchain.git", "Git", probe.first_line.clone())
    } else {
        Check::warn(
            "toolchain.git",
            "Git",
            "git not found or not working",
            "Install Git from https://git-scm.com/downloads",
        )
    }
}

pub(crate) fn wasm_check(targets: &Probe) -> Check {
    const ID: &str = "toolchain.wasm_target";
    const TITLE: &str = "Wasm target";
    let fix = format!("rustup target add {WASM_TARGET}");
    if !targets.found {
        return Check::warn(
            ID,
            TITLE,
            "rustup not found; islands need the Wasm target",
            fix,
        );
    }
    if targets
        .stdout
        .lines()
        .any(|line| line.trim() == WASM_TARGET)
    {
        Check::pass(ID, TITLE, format!("{WASM_TARGET} installed"))
    } else {
        Check::warn(ID, TITLE, format!("{WASM_TARGET} is not installed"), fix)
    }
}

/// One informational line for the optional analysis tools.
pub(crate) fn optional_check(probes: &[Probe]) -> Check {
    let mut installed = Vec::new();
    let mut missing = Vec::new();
    let mut hints = Vec::new();
    for ((name, _, _, hint), probe) in OPTIONAL.iter().zip(probes) {
        if probe.ok() {
            installed.push(*name);
        } else {
            missing.push(*name);
            hints.push(*hint);
        }
    }
    let detail = match (installed.is_empty(), missing.is_empty()) {
        (_, true) => format!("all installed: {}", installed.join(", ")),
        (true, false) => format!("not installed: {}", missing.join(", ")),
        (false, false) => format!(
            "installed: {} · not installed: {}",
            installed.join(", "),
            missing.join(", ")
        ),
    };
    let check = Check::info("toolchain.optional", "Optional tools", detail);
    if hints.is_empty() {
        check
    } else {
        check.with_fix(hints.join("; "))
    }
}

pub(crate) fn cargo_audit_check(probe: &Probe) -> Check {
    if probe.ok() {
        Check::pass(
            "security.cargo_audit",
            "cargo-audit",
            "installed (RustSec advisories)",
        )
    } else {
        Check::warn(
            "security.cargo_audit",
            "cargo-audit",
            "not installed; dependency advisories are not checked",
            "cargo install cargo-audit",
        )
    }
}

/// The toolchain group from already collected probes.
pub(crate) fn checks(probes: &Probes) -> Vec<Check> {
    let mut checks = vec![
        rustc_check(&probes.rustc),
        cargo_check(&probes.cargo),
        components_check(&probes.rustfmt, &probes.clippy),
        git_check(&probes.git),
    ];
    if let Some(targets) = &probes.targets {
        checks.push(wasm_check(targets));
    }
    checks.push(optional_check(&probes.optional));
    checks
}

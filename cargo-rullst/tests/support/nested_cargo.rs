//! One lock around every Cargo command a test runs in the shared target
//! directory.
//!
//! The tests of one binary run in parallel threads, and their nested Cargo
//! commands share the workspace target directory so the framework's
//! dependencies are compiled once. Cargo's build-directory lock makes a
//! second Cargo wait while one compiles, but only while it compiles:
//! `cargo run` gives it up before the program starts, and the commands of
//! two tests interleave. Holding an exclusive lock on one file in the target
//! directory for each whole command runs the nested commands of parallel
//! tests strictly one at a time, in threads or processes, and keeps the warm
//! shared target (a target directory per test would compile every
//! dependency again).

use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::process::{Command, Output};

/// Runs `command` to completion while holding the nested-Cargo lock of
/// `target_root`, the `CARGO_TARGET_DIR` the command uses.
pub fn output(command: &mut Command, target_root: &Path) -> io::Result<Output> {
    fs::create_dir_all(target_root)?;
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(target_root.join(".rullst-nested-cargo.lock"))?;
    // Blocks until no other nested command of this target holds the lock;
    // closing the file when this function returns releases it.
    lock.lock()?;
    command.output()
}

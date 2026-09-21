use super::config::{LinuxConfig, hash_file, hash_tree};
use rullst_labs::{ExecutionProfile, LabError as Error, ReceiptSigner, ToolIdentity};
use std::path::Path;

/// Describes operator-prepared, immutable tool artifacts. The printed hashes
/// pin configuration only; doctor and execution still require actual OS probes.
pub(super) fn run(
    rootfs: &Path,
    launcher: &Path,
    cgroups: &Path,
    seed: &Path,
) -> Result<(), Error> {
    let signer = ReceiptSigner::from_seed(super::service::read_key(seed)?)?;
    let config = LinuxConfig {
        rootfs: rootfs.canonicalize().map_err(|_| Error::Configuration)?,
        launcher: launcher.canonicalize().map_err(|_| Error::Configuration)?,
        cgroups: cgroups.canonicalize().map_err(|_| Error::Configuration)?,
        profile: ExecutionProfile::LinuxExperimental {
            receipt_key: signer.public_key()?,
            tools: ToolIdentity {
                runner: hash_file(&rootfs.join("runner"))?,
                compiler: hash_file(&rootfs.join("toolchain/bin/rustc"))?,
                wasm_toolchain: hash_tree(&rootfs.join("toolchain"))?,
                runtime: hash_tree(&rootfs.join("runtime"))?,
                launcher: hash_file(launcher)?,
                syscall_policy: super::syscalls::fingerprint()?,
                filesystem_policy: super::filesystem::fingerprint()?,
            },
        },
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&config).map_err(|_| Error::Configuration)?
    );
    Ok(())
}

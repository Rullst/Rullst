use rullst_labs::LabError as Error;
use std::{io::Read, os::unix::process::CommandExt, path::Path, process::Command};

/// Trusted inert child. No source, key or job database is passed here. Parent
/// places this PID in its configured cgroup BEFORE sending the one-byte release.
pub(super) fn run(launcher: &Path, rootfs: &Path, group: &Path) -> Result<(), Error> {
    let mut release = [0u8; 1];
    std::io::stdin()
        .read_exact(&mut release)
        .map_err(|_| Error::Protocol)?;
    if release != *b"G" {
        return Err(Error::Protocol);
    }
    let mut command = command(launcher, rootfs, group);
    let _error = command.exec();
    Err(Error::Unsupported)
}
pub(super) fn command(launcher: &Path, rootfs: &Path, group: &Path) -> Command {
    let mut cmd = Command::new(launcher);
    cmd.env_clear();
    cmd.args([
        "--unshare-all",
        "--disable-userns",
        "--assert-userns-disabled",
        "--die-with-parent",
        "--new-session",
        "--cap-drop",
        "ALL",
        "--uid",
        "0",
        "--gid",
        "0",
        "--hostname",
        "rullst-lab",
        "--clearenv",
    ])
    .arg("--ro-bind")
    .arg(rootfs.join("runner"))
    .arg("/runner")
    .arg("--ro-bind")
    .arg(rootfs.join("toolchain"))
    .arg("/toolchain")
    .arg("--ro-bind")
    .arg(rootfs.join("runtime/lib"))
    .arg("/lib")
    .arg("--ro-bind")
    .arg(rootfs.join("runtime/lib64"))
    .arg("/lib64")
    .arg("--ro-bind")
    .arg(group)
    .arg("/limits")
    .args([
        "--proc",
        "/proc",
        "--remount-ro",
        "/proc",
        "--dev",
        "/dev",
        "--size",
        "67108864",
        "--tmpfs",
        "/work",
        "--remount-ro",
        "/",
        "--chdir",
        "/work",
    ]);
    for (key, value) in [
        ("PATH", "/toolchain/bin"),
        ("LD_LIBRARY_PATH", "/toolchain/lib:/lib"),
        ("LANG", "C"),
        ("LC_ALL", "C"),
        ("TMPDIR", "/work"),
        ("RAYON_NUM_THREADS", "1"),
    ] {
        cmd.args(["--setenv", key, value]);
    }
    cmd.args(["--", "/runner", "__worker"]);
    cmd
}

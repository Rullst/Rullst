#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

// Fixed trusted process probes only; no learner source or compiled submission.
// Include the real policy so the regression checks the shipped syscall filter.
#[path = "../src/linux/syscalls.rs"]
mod syscalls;

use std::process::{Command, Stdio};

#[test]
#[ignore = "requires a Linux host that permits seccomp filter installation"]
fn absolute_tool_spawn_preserves_socket_denial() {
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "restricted_tool_spawn_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("RULLST_LABS_SPAWN_PROBE", "1")
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
#[ignore = "isolated helper for absolute_tool_spawn_preserves_socket_denial"]
fn restricted_tool_spawn_child() {
    if std::env::var("RULLST_LABS_SPAWN_PROBE").as_deref() != Ok("1") {
        return;
    }
    let executable = std::env::current_exe().unwrap();
    let directory = executable.parent().unwrap();
    let name = executable.file_name().unwrap();
    assert!(syscalls::fingerprint().is_ok());
    syscalls::apply().unwrap();

    // Rust's changed-PATH + bare-name branch uses socketpair before fork.
    // Reproduce the actual linker-spawn failure without compiling any source.
    let error = Command::new(name)
        .env("PATH", directory)
        .arg("--list")
        .output()
        .unwrap_err();
    assert_eq!(error.raw_os_error(), Some(libc::EPERM));
    let error = std::os::unix::net::UnixStream::pair().unwrap_err();
    assert_eq!(error.raw_os_error(), Some(libc::EPERM));
    let (reader, _writer) = std::io::pipe().unwrap();
    rustix::io::ioctl_fionbio(&reader, true).unwrap();
    rustix::io::ioctl_fionbio(&reader, false).unwrap();
    assert_eq!(
        rustix::io::ioctl_fionread(&reader).unwrap_err(),
        rustix::io::Errno::PERM
    );

    // The same owned executable and environment work with an absolute path.
    let status = Command::new(&executable)
        .env("PATH", directory)
        .arg("--list")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());

    // rustc captures both linker streams with wait_with_output, which needs
    // nonblocking pipe reads on Linux, unlike the null-stream status probe.
    let output = Command::new(&executable)
        .env("PATH", directory)
        .arg("--list")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("restricted_tool_spawn_child")
    );
}

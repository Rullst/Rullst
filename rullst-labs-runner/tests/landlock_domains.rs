#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

// Fixed trusted kernel probes only. No learner source, compilation or Wasm runs.
use landlock::{
    ABI, Access, AccessFs, CompatLevel, Compatible, Ruleset, RulesetAttr, RulesetStatus,
};
use std::{
    fs::File,
    io::Write,
    os::fd::AsRawFd,
    process::{Command, Stdio},
};

fn restrict() {
    let status = Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(ABI::V3))
        .unwrap()
        .create()
        .unwrap()
        .restrict_self()
        .unwrap();
    assert_eq!(status.ruleset, RulesetStatus::FullyEnforced);
    assert!(status.no_new_privs);
}

#[test]
#[ignore = "requires a Linux host with Landlock ABI 3; mandatory isolated acceptance invokes it"]
fn independent_domains_block_interpreter_descriptors() {
    // Test harnesses use a thread: target that exact task's domain, not the
    // unsandboxed harness leader's domain. Use an owned anonymous pipe even
    // when Cargo's own stdout is redirected to a regular log file.
    let parent_task = std::fs::read_link("/proc/thread-self").unwrap();
    let (_owned_read, owned_write) = std::io::pipe().unwrap();
    let parent_pipe = format!(
        "{}/fd/{}",
        parent_task.to_str().unwrap(),
        owned_write.as_raw_fd()
    );
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "compiler_domain_probe",
            "--nocapture",
        ])
        .env("RULLST_TEST_PARENT_TASK", parent_pipe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    // The compiler was spawned outside this new domain; it will create a
    // separate domain after receiving the fixed synchronization byte.
    restrict();
    input.write_all(b"G").unwrap();
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "owned child of the independent-domain kernel regression"]
fn compiler_domain_probe() {
    use std::io::Read;
    use std::os::unix::fs::FileTypeExt;
    let task = std::env::var("RULLST_TEST_PARENT_TASK").unwrap();
    assert!(task.split('/').all(|part| matches!(part, "task" | "fd")
        || (!part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))));
    let parent_pipe = format!("/proc/{task}");
    let mut release = [0; 1];
    std::io::stdin().read_exact(&mut release).unwrap();
    assert_eq!(release, *b"G");
    // Prove the denial below is caused by the new domain, not a missing path.
    assert!(
        File::open(&parent_pipe)
            .unwrap()
            .metadata()
            .unwrap()
            .file_type()
            .is_fifo()
    );
    restrict();
    assert!(
        matches!(File::open(parent_pipe), Err(e) if matches!(e.raw_os_error(), Some(libc::EACCES | libc::EPERM)))
    );
    assert!(
        matches!(File::open("/proc/self/status"), Err(e) if e.raw_os_error() == Some(libc::EACCES))
    );
    // The documented own-pipe exception still exists: never claim path-based
    // Landlock rules alone block anonymous descriptor aliases.
    assert!(
        File::open("/proc/self/fd/1")
            .unwrap()
            .metadata()
            .unwrap()
            .file_type()
            .is_fifo()
    );
}

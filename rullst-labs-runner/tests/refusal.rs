#[test]
fn unsupported_direct_worker_never_requests_or_executes_student_input() {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_rullst-labs-runner"))
        .arg("__worker")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            panic!("unsupported worker did not fail closed");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unsupported or unenforced")
            || String::from_utf8_lossy(&output.stderr).contains("invalid lab configuration")
    );
}

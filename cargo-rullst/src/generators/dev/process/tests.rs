use super::super::build::compile_in;
use super::*;

fn write_fixture(root: &Path, port_file: &Path, body: &str) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='reload-audit-fixture'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    let source = format!(
        r#"
use std::io::{{Read, Write}};
fn main() {{
    if std::env::args().any(|arg| arg == "db:migrate") {{ return; }}
    // The supervisor must clear legacy activation, including inherited values.
    assert!(std::env::var_os("HOT_RELOAD").is_none());
    let generation = std::env::var("RULLST_DEV_GENERATION").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    std::fs::write({port_file:?}, listener.local_addr().unwrap().port().to_string()).unwrap();
    for stream in listener.incoming() {{
        let mut stream = stream.unwrap();
        let mut request = [0u8; 1024];
        let _ = stream.read(&mut request);
        let body = {body:?};
        let _ = write!(stream, "HTTP/1.1 200 OK\r\nConnection: close\r\nx-rullst-dev-generation: {{generation}}\r\nContent-Length: {{}}\r\n\r\n{{body}}", body.len());
    }}
}}
"#
    );
    fs::write(root.join("src/main.rs"), source).unwrap();
}

async fn port_from(path: &Path) -> u16 {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(value) = fs::read_to_string(path)
            && let Ok(port) = value.parse()
        {
            return port;
        }
        assert!(Instant::now() < deadline, "fixture did not report its port");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[tokio::test]
async fn failed_build_keeps_previous_process_then_fixed_build_restarts_and_reaps() {
    let temp = tempfile::tempdir().unwrap();
    let port_file = temp.path().join("port");
    write_fixture(temp.path(), &port_file, "old application");
    let executable = compile_in(temp.path()).await.unwrap();
    let mut app = Application::prepare(&executable).unwrap();
    let old_snapshot = app.executable.clone();
    let (logs, _rx) = mpsc::channel(8);
    app.migrate(false, &logs).await.unwrap();
    app.start(false, &logs).unwrap();
    let port = port_from(&port_file).await;
    app.wait_ready(port).await.unwrap();
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    fs::write(temp.path().join("src/main.rs"), "this is not valid Rust").unwrap();
    let error = compile_in(temp.path()).await.unwrap_err();
    assert!(error.to_string().contains("Cargo build failed"));
    assert!(app.try_wait().unwrap().is_none());
    assert_eq!(
        client
            .get(format!("http://127.0.0.1:{port}/"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "old application"
    );
    // A missing/locked/disk-full snapshot must not tear down the good child.
    assert!(
        super::super::prepare_replacement(&temp.path().join("missing-binary"), false, &logs)
            .is_none()
    );
    assert!(app.try_wait().unwrap().is_none());

    write_fixture(temp.path(), &port_file, "new application");
    let next = compile_in(temp.path()).await.unwrap();
    let mut replacement = Application::prepare(&next).unwrap();
    assert_ne!(replacement.executable, old_snapshot);
    app.stop().unwrap();
    // Validate termination before the OS can reuse this port for the next child.
    assert!(
        client
            .get(format!("http://127.0.0.1:{port}/"))
            .send()
            .await
            .is_err()
    );
    drop(app);
    assert!(!old_snapshot.exists());
    fs::remove_file(&port_file).unwrap();
    replacement.start(false, &logs).unwrap();
    let port = port_from(&port_file).await;
    replacement.wait_ready(port).await.unwrap();
    assert_eq!(
        client
            .get(format!("http://127.0.0.1:{port}/"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "new application"
    );
    let path = replacement.executable.clone();
    drop(replacement);
    assert!(!path.exists());
    assert!(
        client
            .get(format!("http://127.0.0.1:{port}/"))
            .send()
            .await
            .is_err()
    );
}

#[tokio::test]
async fn readiness_requires_our_generation_not_an_arbitrary_open_port() {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let serving = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buffer = [0; 1024];
            let _ = stream.read(&mut buffer).await;
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\nx-rullst-dev-generation: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\r\n\r\n").await;
        }
    });
    // No actual child: only the readiness protocol is under test here.
    let temp = tempfile::tempdir().unwrap();
    let mut app = Application {
        child: None,
        executable: temp.path().join("unused"),
        generation: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
        group: None,
        exit_status: None,
    };
    let result = tokio::time::timeout(Duration::from_millis(600), app.wait_ready(port)).await;
    assert!(
        result.is_err(),
        "an unrelated service must never confirm this generation"
    );
    serving.abort();
}

#[tokio::test]
async fn restarting_the_same_snapshot_has_a_new_process_generation() {
    let temp = tempfile::tempdir().unwrap();
    let port_file = temp.path().join("port");
    write_fixture(temp.path(), &port_file, "same binary");
    let executable = compile_in(temp.path()).await.unwrap();
    let mut app = Application::prepare(&executable).unwrap();
    let (logs, _rx) = mpsc::channel(8);
    app.start(false, &logs).unwrap();
    let first = app.generation.clone();
    app.wait_ready(port_from(&port_file).await).await.unwrap();
    app.stop().unwrap();
    fs::remove_file(&port_file).unwrap();
    app.start(false, &logs).unwrap();
    assert_ne!(first, app.generation);
    app.wait_ready(port_from(&port_file).await).await.unwrap();
}

#[cfg(unix)]
fn active_process(pid: u32) -> bool {
    ProcessGroup::new(pid)
        .exit_observed()
        .is_ok_and(|exited| !exited)
}

#[cfg(unix)]
async fn assert_process_stopped(pid: u32) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while active_process(pid) && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        !active_process(pid),
        "owned descendant {pid} survived cleanup"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn parent_first_exit_cleans_descendants_before_reaping_and_disarms_the_group() {
    let mut command = Command::new("/bin/sh");
    command
        .args([
            "-c",
            "sleep 60 </dev/null >/dev/null 2>&1 & printf '%s' \"$!\"",
        ])
        .stdout(Stdio::piped());
    configure_group(&mut command);
    let mut child = command.spawn().unwrap();
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    let descendant: u32 = output.parse().unwrap();
    let mut app = Application {
        group: Some(ProcessGroup::new(child.id())),
        child: Some(child),
        executable: PathBuf::new(),
        generation: String::new(),
        exit_status: None,
    };
    assert!(active_process(descendant));
    let deadline = Instant::now() + Duration::from_secs(3);
    while app.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_process_stopped(descendant).await;
    assert!(app.try_wait().unwrap().unwrap().success());
    // A later stop must only reap/cache, not signal the retired group again.
    app.stop().unwrap();
    assert!(app.group.is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn cancelling_a_real_migration_kills_its_owned_descendant_and_drains_bounded_logs() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("migration-script");
    let pid_file = temp.path().join("worker-pid");
    // Paths are supplied by tempfile, not user input; quote the argument safely.
    let script = format!(
        "#!/bin/sh\nsleep 60 </dev/null >/dev/null 2>&1 &\nprintf '%s' \"$!\" > '{}'\nwhile :; do printf 'bounded migration output\\n'; done\n",
        pid_file.display()
    );
    fs::write(&source, script).unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
    let app = Application::prepare(&source).unwrap();
    let (logs, mut rx) = mpsc::channel(1);
    let descendant = {
        let migration = app.migrate(true, &logs);
        tokio::pin!(migration);
        tokio::select! {
            result = &mut migration => panic!("fixture unexpectedly exited: {result:?}"),
            descendant = async {
                let deadline = Instant::now() + Duration::from_secs(3);
                loop {
                    if let Ok(value) = fs::read_to_string(&pid_file) && let Ok(pid) = value.parse::<u32>() && !rx.is_empty() { break pid; }
                    assert!(Instant::now() < deadline);
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            } => descendant,
        }
        // The migration future is dropped here, as on dashboard exit.
    };
    assert_process_stopped(descendant).await;
    assert!(rx.len() <= 1);
    assert!(matches!(rx.try_recv(), Ok(LogMsg::AppStdout(line)) if line.len() <= 4096));
}

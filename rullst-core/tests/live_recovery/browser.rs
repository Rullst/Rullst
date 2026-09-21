use super::app;
use serde_json::json;
use std::{net::SocketAddr, path::Path, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
};

struct Worker {
    child: Child,
    address: SocketAddr,
}

impl Worker {
    async fn start(database: &str, port: u16, directory: &Path) -> Self {
        let ready = directory.join(format!("ready-{}", uuid::Uuid::new_v4().simple()));
        let config = app::Configuration {
            database: database.to_owned(),
            port,
            ready: ready.clone(),
        };
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "fixture_server", "--nocapture"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&config).unwrap())
            .await
            .unwrap();
        let address = tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                assert!(
                    child.try_wait().unwrap().is_none(),
                    "owned Live server exited before readiness"
                );
                if let Ok(value) = tokio::fs::read_to_string(&ready).await
                    && let Ok(address) = value.parse::<SocketAddr>()
                {
                    break address;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .unwrap();
        Self { child, address }
    }
    async fn stop(mut self) {
        self.child.kill().await.unwrap();
        self.child.wait().await.unwrap();
    }
}

pub async fn exercise() {
    let directory = tempfile::tempdir().unwrap();
    let database = format!(
        "sqlite:{}?mode=rwc",
        directory.path().join("live.db").to_string_lossy()
    );
    let worker = Worker::start(&database, 0, directory.path()).await;
    let address = worker.address;
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/live-recovery-browser.mjs");
    let mut browser = Command::new("node")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = browser.stdin.take().unwrap();
    let configuration = json!({"origin":format!("http://{address}"),"teacher":app::TEACHER,
        "freshTeacher":app::FRESH_TEACHER,"learner":app::LEARNER,"other":app::OTHER,"csrf":app::CSRF});
    input
        .write_all(format!("{configuration}\n").as_bytes())
        .await
        .unwrap();
    let mut output = BufReader::new(browser.stdout.take().unwrap()).lines();
    let mut worker = Some(worker);
    let mut restarts = 0;
    tokio::time::timeout(Duration::from_secs(150), async {
        loop {
            let line = output
                .next_line()
                .await
                .unwrap()
                .expect("browser must emit its final receipt");
            assert!(line.len() <= 4096);
            let message: serde_json::Value = serde_json::from_str(&line).unwrap();
            match message["command"].as_str().unwrap() {
                "restart" => {
                    assert_eq!(restarts, 0);
                    worker.take().unwrap().stop().await;
                    worker = Some(Worker::start(&database, address.port(), directory.path()).await);
                    restarts += 1;
                    input.write_all(b"{\"restarted\":true}\n").await.unwrap();
                }
                "passed" => break,
                _ => panic!("unknown owned browser request"),
            }
        }
    })
    .await
    .expect("browser recovery journey deadline");
    assert_eq!(restarts, 1, "an actual application process must restart");
    drop(input);
    assert!(
        tokio::time::timeout(Duration::from_secs(15), browser.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    worker.take().unwrap().stop().await;
}

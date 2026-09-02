use rullst_messaging::BrokerConfig;
use std::path::{Path, PathBuf};

pub fn fixture(label: &str) -> (PathBuf, String) {
    let directory = PathBuf::from("target").join("rullst-messaging-tests");
    std::fs::create_dir_all(&directory).expect("create messaging fixture directory");
    let path = directory.join(format!("{label}-{}.sqlite", uuid::Uuid::new_v4().simple()));
    let url = format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"));
    (path, url)
}

pub fn config(namespace: &str) -> BrokerConfig {
    config_with_limits(namespace, 64, 16, 3, 4 * 1024)
}

pub fn config_with_limits(
    namespace: &str,
    retained: usize,
    subscriptions: usize,
    attempts: u32,
    payload_bytes: usize,
) -> BrokerConfig {
    BrokerConfig::try_new(namespace)
        .expect("valid config")
        .with_limits(retained, subscriptions, attempts, payload_bytes)
        .expect("valid limits")
}

pub fn cleanup(path: &Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

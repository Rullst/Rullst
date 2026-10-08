//! What gets measured: a loopback-only URL, the process listening on it, or
//! the project's release binary started on a free loopback port.
use std::collections::VecDeque;
use std::io::{self, Read};
use std::net::IpAddr;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const STDERR_TAIL_BYTES: usize = 4096;
const READY_TIMEOUT: Duration = Duration::from_secs(60);
const STOP_GRACE: Duration = Duration::from_secs(5);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum UrlError {
    #[error("`{0}` is not a valid URL")]
    Invalid(String),
    #[error("only http:// URLs are supported, such as http://127.0.0.1:3000")]
    Scheme,
    #[error(
        "footprint measures loopback addresses only (127.0.0.0/8, ::1 or localhost); `{0}` is not loopback"
    )]
    NotLoopback(String),
    #[error("the URL must not contain credentials, a query or a fragment")]
    Extra,
    #[error("pass the request path with --path, not in --url")]
    Path,
}

/// Accepts `http://` origins on loopback only; `localhost` becomes
/// `127.0.0.1` so name resolution can never leave the machine.
pub(crate) fn validate_url(raw: &str) -> Result<reqwest::Url, UrlError> {
    let mut url =
        reqwest::Url::parse(raw.trim()).map_err(|_| UrlError::Invalid(raw.to_string()))?;
    if url.scheme() != "http" {
        return Err(UrlError::Scheme);
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(UrlError::Extra);
    }
    if url.path() != "/" {
        return Err(UrlError::Path);
    }
    let host = url.host_str().unwrap_or_default().to_string();
    if host.eq_ignore_ascii_case("localhost") {
        url.set_host(Some("127.0.0.1"))
            .map_err(|_| UrlError::Invalid(raw.to_string()))?;
        return Ok(url);
    }
    let address: Option<IpAddr> = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse()
        .ok();
    let loopback = match address {
        Some(IpAddr::V4(address)) => address.is_loopback(),
        Some(IpAddr::V6(address)) => {
            address.is_loopback() || address.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback())
        }
        None => false,
    };
    if !loopback {
        return Err(UrlError::NotLoopback(host));
    }
    Ok(url)
}

/// `base` with `path` (and its optional query); the host never changes.
pub(super) fn request_url(base: &reqwest::Url, path: &str) -> reqwest::Url {
    let mut url = base.clone();
    let (path, query) = path
        .split_once('?')
        .map_or((path, None), |(p, q)| (p, Some(q)));
    url.set_path(path);
    url.set_query(query);
    url
}

/// Inodes of sockets listening on `port` in a `/proc/net/tcp{,6}` table.
pub(super) fn listening_inodes(table: &str, port: u16) -> Vec<u64> {
    const LISTEN: &str = "0A";
    table
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let (_, local_port) = fields.get(1)?.rsplit_once(':')?;
            let listening = *fields.get(3)? == LISTEN;
            let inode: u64 = fields.get(9)?.parse().ok()?;
            (listening && u16::from_str_radix(local_port, 16).ok()? == port && inode != 0)
                .then_some(inode)
        })
        .collect()
}

/// The process found listening on the URL's port.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Listener {
    Found(u32),
    Unknown(&'static str),
}

/// Finds the one process listening on `port` by matching socket inodes
/// against `/proc/<pid>/fd` links (Linux; same-user processes only).
pub(super) fn listening_process(port: u16) -> Listener {
    if !cfg!(target_os = "linux") {
        return Listener::Unknown("process lookup by port uses /proc and is Linux-only");
    }
    let mut inodes = Vec::new();
    for table in ["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(text) = std::fs::read_to_string(table) {
            inodes.extend(listening_inodes(&text, port));
        }
    }
    if inodes.is_empty() {
        return Listener::Unknown("no listening socket found for the port in /proc/net/tcp");
    }
    let targets: Vec<String> = inodes
        .iter()
        .map(|inode| format!("socket:[{inode}]"))
        .collect();
    let mut owners = Vec::new();
    let Ok(processes) = std::fs::read_dir("/proc") else {
        return Listener::Unknown("/proc is not readable");
    };
    for entry in processes.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        // Other users' descriptors are unreadable and skipped.
        let Ok(descriptors) = std::fs::read_dir(entry.path().join("fd")) else {
            continue;
        };
        let owns = descriptors.flatten().any(|descriptor| {
            std::fs::read_link(descriptor.path()).is_ok_and(|link| {
                targets
                    .iter()
                    .any(|target| link.as_os_str() == target.as_str())
            })
        });
        if owns && !owners.contains(&pid) {
            owners.push(pid);
        }
    }
    match owners.as_slice() {
        [pid] => Listener::Found(*pid),
        [] => Listener::Unknown("the listening process belongs to another user or is not visible"),
        _ => Listener::Unknown("several processes share the listening socket"),
    }
}

/// A free loopback port; another process could take it before the app binds.
pub(super) fn free_port() -> io::Result<u16> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

/// The release binary started by `footprint`, stopped when dropped.
pub(super) struct OwnedApp {
    child: Child,
    stderr: Arc<Mutex<VecDeque<u8>>>,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum StartError {
    #[error("the app exited during start-up ({0})")]
    Exited(ExitStatus),
    #[error("the app did not answer on {0} within 60 seconds")]
    Timeout(String),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl OwnedApp {
    /// Starts `binary` in production mode on `127.0.0.1:port`, in the
    /// current directory. It shares the terminal's process group so Ctrl+C
    /// stops it too.
    pub(super) fn spawn(binary: &Path, port: u16) -> io::Result<Self> {
        let mut child = Command::new(binary)
            .env("RULLST_ENV", "production")
            .env("HOST", "127.0.0.1")
            .env("PORT", port.to_string())
            .env_remove("HOT_RELOAD")
            .env_remove("RULLST_HMR_TOKEN")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        let stderr = Arc::new(Mutex::new(VecDeque::with_capacity(STDERR_TAIL_BYTES)));
        if let Some(mut pipe) = child.stderr.take() {
            let tail = Arc::clone(&stderr);
            std::thread::spawn(move || {
                let mut buffer = [0_u8; 1024];
                while let Ok(count) = pipe.read(&mut buffer) {
                    if count == 0 {
                        break;
                    }
                    if let Ok(mut tail) = tail.lock() {
                        tail.extend(&buffer[..count]);
                        let excess = tail.len().saturating_sub(STDERR_TAIL_BYTES);
                        tail.drain(..excess);
                    }
                }
            });
        }
        Ok(Self { child, stderr })
    }

    pub(super) fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Waits until `url` answers with any HTTP status.
    pub(super) async fn wait_ready(
        &mut self,
        client: &reqwest::Client,
        url: &reqwest::Url,
    ) -> Result<(), StartError> {
        let deadline = Instant::now() + READY_TIMEOUT;
        while Instant::now() < deadline {
            if let Some(status) = self.child.try_wait()? {
                return Err(StartError::Exited(status));
            }
            let probe = client.get(url.clone()).timeout(Duration::from_secs(1));
            if probe.send().await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(StartError::Timeout(url.to_string()))
    }

    /// The last lines the app wrote to stderr, redacted.
    pub(super) fn stderr_tail(&self) -> String {
        let bytes: Vec<u8> = self
            .stderr
            .lock()
            .map(|tail| tail.iter().copied().collect())
            .unwrap_or_default();
        String::from_utf8_lossy(&bytes)
            .lines()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(crate::ui::error_report::sanitize)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// SIGTERM, a short grace period, then a kill.
    pub(super) fn stop(&mut self) {
        if matches!(self.child.try_wait(), Ok(Some(_))) {
            return;
        }
        #[cfg(unix)]
        if let Some(pid) = i32::try_from(self.child.id())
            .ok()
            .and_then(rustix::process::Pid::from_raw)
        {
            let _ = rustix::process::kill_process(pid, rustix::process::Signal::TERM);
            let deadline = Instant::now() + STOP_GRACE;
            while matches!(self.child.try_wait(), Ok(None)) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

impl Drop for OwnedApp {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_loopback_http_origins_are_accepted() {
        for (raw, normalized) in [
            ("http://127.0.0.1:3000", "http://127.0.0.1:3000/"),
            ("http://127.10.0.1:8080/", "http://127.10.0.1:8080/"),
            ("http://[::1]:3000", "http://[::1]:3000/"),
            ("http://LOCALHOST:3000", "http://127.0.0.1:3000/"),
            (
                "http://[::ffff:127.0.0.1]:3000",
                "http://[::ffff:7f00:1]:3000/",
            ),
        ] {
            assert_eq!(validate_url(raw).unwrap().as_str(), normalized, "{raw}");
        }
        for (raw, error) in [
            (
                "http://example.com",
                UrlError::NotLoopback("example.com".into()),
            ),
            (
                "http://10.0.0.1:80",
                UrlError::NotLoopback("10.0.0.1".into()),
            ),
            (
                "http://0.0.0.0:3000",
                UrlError::NotLoopback("0.0.0.0".into()),
            ),
            ("http://[::]:3000", UrlError::NotLoopback("[::]".into())),
            (
                "http://localhost.evil.example",
                UrlError::NotLoopback("localhost.evil.example".into()),
            ),
            ("https://127.0.0.1:3000", UrlError::Scheme),
            ("ftp://127.0.0.1", UrlError::Scheme),
            ("http://user:secret@127.0.0.1:3000", UrlError::Extra),
            ("http://127.0.0.1:3000/?q=1", UrlError::Extra),
            ("http://127.0.0.1:3000/#x", UrlError::Extra),
            ("http://127.0.0.1:3000/health", UrlError::Path),
            ("localhost:3000", UrlError::Scheme),
            ("127.0.0.1:3000", UrlError::Invalid("127.0.0.1:3000".into())),
            ("not a url", UrlError::Invalid("not a url".into())),
        ] {
            assert_eq!(validate_url(raw), Err(error), "{raw}");
        }
    }

    #[test]
    fn request_paths_keep_the_loopback_host() {
        let base = validate_url("http://127.0.0.1:3000").unwrap();
        assert_eq!(
            request_url(&base, "/search?q=rust").as_str(),
            "http://127.0.0.1:3000/search?q=rust"
        );
        let odd = request_url(&base, "/@evil.example/x");
        assert_eq!(odd.host_str(), Some("127.0.0.1"));
    }

    #[test]
    fn listening_sockets_are_found_by_port_and_state() {
        let table = include_str!("fixtures/proc_net_tcp");
        assert_eq!(listening_inodes(table, 3000), [424242]);
        // Port 8080 only has an established connection.
        assert!(listening_inodes(table, 8080).is_empty());
        assert!(listening_inodes(table, 9).is_empty());
        let table6 = include_str!("fixtures/proc_net_tcp6");
        assert_eq!(listening_inodes(table6, 3000), [515151]);
    }

    #[test]
    fn a_free_port_is_a_bindable_loopback_port() {
        let port = free_port().unwrap();
        assert_ne!(port, 0);
    }
}

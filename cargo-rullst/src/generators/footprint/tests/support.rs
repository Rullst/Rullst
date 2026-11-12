//! Fixtures shared by the footprint tests: a loopback HTTP server, executable
//! scripts and child processes waited on with a deadline.
// The end-to-end tests that use these helpers measure /proc and run on Linux
// only; on macOS and Windows the helpers are compiled but unused.
#![cfg_attr(not(target_os = "linux"), allow(dead_code, unused_imports))]
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// One request the test server received.
#[derive(Clone, Debug)]
pub(in crate::generators::footprint) struct Request {
    pub(in crate::generators::footprint) path: String,
    /// The request line and headers.
    pub(in crate::generators::footprint) head: String,
}

impl Request {
    /// The value of header `name`, matched without regard to case.
    pub(in crate::generators::footprint) fn header(&self, name: &str) -> Option<&str> {
        self.head.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim().eq_ignore_ascii_case(name).then(|| value.trim())
        })
    }
}

/// The raw response to a request, or `None` to close without an answer.
pub(in crate::generators::footprint) type Handler =
    Arc<dyn Fn(&Request) -> Option<Vec<u8>> + Send + Sync>;

/// A loopback HTTP/1.1 server on a port of its own. By default `/missing`
/// answers 404, `/redirect` a 302 to `/target`, `/hangup` closes the
/// connection without an answer, `/compressed` sends a body labelled
/// `Content-Encoding: br` and every other path 200. Each connection carries
/// one request.
pub(in crate::generators::footprint) struct TestServer {
    pub(in crate::generators::footprint) base: reqwest::Url,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl TestServer {
    pub(in crate::generators::footprint) fn start() -> Self {
        Self::with(Arc::new(default_answer))
    }

    /// A server that answers with `handler`.
    pub(in crate::generators::footprint) fn with(handler: Handler) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let base = reqwest::Url::parse(&format!("http://{address}/")).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&requests);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let seen = Arc::clone(&seen);
                let handler = Arc::clone(&handler);
                std::thread::spawn(move || answer(stream, &seen, handler.as_ref()));
            }
        });
        Self { base, requests }
    }

    pub(in crate::generators::footprint) fn url(&self, path: &str) -> reqwest::Url {
        self.base.join(path).unwrap()
    }

    /// The requested paths so far, in order of arrival.
    pub(in crate::generators::footprint) fn paths(&self) -> Vec<String> {
        self.requests()
            .into_iter()
            .map(|request| request.path)
            .collect()
    }

    /// The requests so far, in order of arrival.
    pub(in crate::generators::footprint) fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

/// A complete response with `headers` (each ending in `\r\n`) and `body`.
pub(in crate::generators::footprint) fn response(
    status: &str,
    headers: &str,
    body: &[u8],
) -> Option<Vec<u8>> {
    let mut bytes = format!(
        "HTTP/1.1 {status}\r\n{headers}content-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    bytes.extend_from_slice(body);
    Some(bytes)
}

fn default_answer(request: &Request) -> Option<Vec<u8>> {
    match request.path.as_str() {
        "/hangup" => None,
        "/missing" => response("404 Not Found", "", b"missing"),
        "/redirect" => response("302 Found", "location: /target\r\n", b""),
        "/compressed" => response("200 OK", "content-encoding: br\r\n", b"BROTL"),
        _ => response("200 OK", "", b"ok"),
    }
}

fn answer(
    mut stream: TcpStream,
    seen: &Mutex<Vec<Request>>,
    handler: &(dyn Fn(&Request) -> Option<Vec<u8>> + Send + Sync),
) {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut head = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(count) => head.extend_from_slice(&buffer[..count]),
        }
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let path = head
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_string();
    let request = Request { path, head };
    seen.lock().unwrap().push(request.clone());
    if let Some(response) = handler(&request) {
        let _ = stream.write_all(&response);
    }
}

/// Writes an executable `#!/bin/sh` script. Its first line exits at once
/// for `--warm-up`: another test's fork may briefly hold the new file open
/// for writing (ETXTBSY), so the script is run until it starts.
#[cfg(unix)]
pub(in crate::generators::footprint) fn script(
    directory: &Path,
    name: &str,
    body: &str,
) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = directory.join(name);
    std::fs::write(
        &path,
        format!("#!/bin/sh\n[ \"$1\" != --warm-up ] || exit 0\n{body}\n"),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match Command::new(&path).arg("--warm-up").status() {
            Err(error) if error.raw_os_error() == Some(26) && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            result => {
                assert!(result.unwrap().success(), "{name} did not start");
                return path;
            }
        }
    }
}

/// The `sleep` program, for scripts that run with a `PATH` of their own.
#[cfg(unix)]
pub(in crate::generators::footprint) fn sleep_program() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|directory| directory.join("sleep"))
        .find(|candidate| candidate.is_file())
        .unwrap()
}

/// Runs `command` with its output captured, killing it after `limit` so a
/// mutant that never returns fails the test instead of hanging it.
pub(in crate::generators::footprint) fn output_within(
    mut command: Command,
    limit: Duration,
) -> Output {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = drain(&mut child, true);
    let stderr = drain(&mut child, false);
    let deadline = Instant::now() + limit;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("the child process ran longer than {limit:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    Output {
        status,
        stdout: stdout.join().unwrap(),
        stderr: stderr.join().unwrap(),
    }
}

fn drain(child: &mut Child, stdout: bool) -> std::thread::JoinHandle<Vec<u8>> {
    let mut pipe: Box<dyn Read + Send> = if stdout {
        Box::new(child.stdout.take().unwrap())
    } else {
        Box::new(child.stderr.take().unwrap())
    };
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        bytes
    })
}

/// The text a child test printed between `<<<{tag}` and `{tag}>>>` lines.
pub(in crate::generators::footprint) fn between_markers(stdout: &[u8], tag: &str) -> String {
    let stdout = String::from_utf8_lossy(stdout);
    stdout
        .split(&format!("<<<{tag}\n"))
        .nth(1)
        .and_then(|rest| rest.split(&format!("{tag}>>>\n")).next())
        .unwrap_or_else(|| panic!("no {tag} markers in:\n{stdout}"))
        .to_string()
}

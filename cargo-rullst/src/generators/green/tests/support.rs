//! Fixtures shared by the green tests: a loopback HTTP server, executable
//! scripts and child processes waited on with a deadline.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A loopback HTTP/1.1 server on a port of its own. `/missing` answers 404,
/// `/redirect` a 302 to `/target`, `/hangup` closes the connection without
/// an answer and every other path 200. Each connection carries one request.
pub(in crate::generators::green) struct TestServer {
    pub(in crate::generators::green) base: reqwest::Url,
    paths: Arc<Mutex<Vec<String>>>,
}

impl TestServer {
    pub(in crate::generators::green) fn start() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let base = reqwest::Url::parse(&format!("http://{address}/")).unwrap();
        let paths = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&paths);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let seen = Arc::clone(&seen);
                std::thread::spawn(move || answer(stream, &seen));
            }
        });
        Self { base, paths }
    }

    pub(in crate::generators::green) fn url(&self, path: &str) -> reqwest::Url {
        self.base.join(path).unwrap()
    }

    /// The requested paths so far, in order of arrival.
    pub(in crate::generators::green) fn paths(&self) -> Vec<String> {
        self.paths.lock().unwrap().clone()
    }
}

fn answer(mut stream: TcpStream, seen: &Mutex<Vec<String>>) {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(count) => request.extend_from_slice(&buffer[..count]),
        }
    }
    let text = String::from_utf8_lossy(&request);
    let path = text
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_string();
    seen.lock().unwrap().push(path.clone());
    let response = match path.as_str() {
        "/hangup" => return,
        "/missing" => {
            "HTTP/1.1 404 Not Found\r\ncontent-length: 7\r\nconnection: close\r\n\r\nmissing"
        }
        "/redirect" => {
            "HTTP/1.1 302 Found\r\nlocation: /target\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
        }
        _ => "HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\nok",
    };
    let _ = stream.write_all(response.as_bytes());
}

/// Writes an executable `#!/bin/sh` script. Its first line exits at once
/// for `--warm-up`: another test's fork may briefly hold the new file open
/// for writing (ETXTBSY), so the script is run until it starts.
#[cfg(unix)]
pub(in crate::generators::green) fn script(directory: &Path, name: &str, body: &str) -> PathBuf {
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
pub(in crate::generators::green) fn sleep_program() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|directory| directory.join("sleep"))
        .find(|candidate| candidate.is_file())
        .unwrap()
}

/// Runs `command` with its output captured, killing it after `limit` so a
/// mutant that never returns fails the test instead of hanging it.
pub(in crate::generators::green) fn output_within(mut command: Command, limit: Duration) -> Output {
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
pub(in crate::generators::green) fn between_markers(stdout: &[u8], tag: &str) -> String {
    let stdout = String::from_utf8_lossy(stdout);
    stdout
        .split(&format!("<<<{tag}\n"))
        .nth(1)
        .and_then(|rest| rest.split(&format!("{tag}>>>\n")).next())
        .unwrap_or_else(|| panic!("no {tag} markers in:\n{stdout}"))
        .to_string()
}

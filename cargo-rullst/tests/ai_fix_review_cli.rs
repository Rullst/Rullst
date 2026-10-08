//! `cargo rullst ai fix` and `cargo rullst ai review` through the real binary
//! with the offline assistant and scripted (non-terminal) standard input.
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const PROVIDER_VARIABLES: [&str; 8] = [
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "GEMINI_API_KEY",
    "DEEPSEEK_API_KEY",
    "OLLAMA_HOST",
    "RULLST_AI_BASE_URL",
    "RULLST_AI_MODEL",
    "OPENAI_BASE_URL",
];
const ID: &str = "0123456789abcdef0123456789abcdef";

struct Sandbox {
    _directory: tempfile::TempDir,
    home: PathBuf,
    project: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(directory.path()).unwrap();
        let home = root.join("home");
        let project = root.join("app");
        fs::create_dir_all(home.join(".config")).unwrap();
        fs::create_dir_all(project.join("src")).unwrap();
        fs::write(
            project.join("Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::write(project.join("src/main.rs"), "fn main() {}\n").unwrap();
        Self {
            _directory: directory,
            home,
            project,
        }
    }

    fn command(&self, cwd: &Path, program: &str, args: &[&str]) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(cwd)
            .args(args)
            .env("RULLST_UPDATE_CHECK", "0")
            .env("NO_COLOR", "1")
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env("APPDATA", self.home.join(".config"))
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for name in PROVIDER_VARIABLES {
            command.env_remove(name);
        }
        command
    }

    fn rullst(&self, cwd: &Path, args: &[&str]) -> Output {
        self.command(cwd, env!("CARGO_BIN_EXE_rullst"), args)
            .output()
            .unwrap()
    }

    fn git(&self, args: &[&str]) -> bool {
        self.command(&self.project, "git", args)
            .output()
            .is_ok_and(|output| output.status.success())
    }
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn review_reports_json_without_secrets_or_protected_files() {
    let sandbox = Sandbox::new();
    if !sandbox.git(&["init", "-q"]) {
        return;
    }
    fs::write(sandbox.project.join(".env"), "PORT=3000\n").unwrap();
    assert!(sandbox.git(&["add", "."]));
    assert!(sandbox.git(&["commit", "-q", "--no-gpg-sign", "-m", "init"]));
    let secret = format!("{}_{}", "ghp", "a1B2".repeat(9));
    fs::write(
        sandbox.project.join("src/main.rs"),
        format!(
            "fn main() {{\n    let token = \"{secret}\";\n    std::env::var(\"X\").unwrap();\n}}\n"
        ),
    )
    .unwrap();
    fs::write(sandbox.project.join(".env"), format!("TOKEN={secret}\n")).unwrap();

    let output = sandbox.rullst(&sandbox.project, &["ai", "review", "--json"]);
    assert!(output.status.success(), "{}", text(&output));
    assert!(!text(&output).contains(&secret));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema"], "rullst.ai-review.v1");
    assert_eq!(report["offline"], true);
    assert_eq!(
        report["files"]["reviewed"],
        serde_json::json!(["src/main.rs"])
    );
    assert_eq!(report["files"]["omitted"][0]["path"], ".env");
    assert_eq!(report["redactions"], 1);
    let findings = report["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 2, "{report}");
    assert_eq!(findings[0]["severity"], "high");
    assert_eq!(findings[0]["line"], 2);
    assert_eq!(findings[1]["severity"], "medium");
    assert_eq!(findings[1]["line"], 3);

    let plain = sandbox.rullst(&sandbox.project, &["ai", "review"]);
    assert!(plain.status.success(), "{}", text(&plain));
    let plain = text(&plain);
    assert!(plain.contains("Omitted, never sent: .env"), "{plain}");
    assert!(plain.contains("[MEDIUM] src/main.rs:3"), "{plain}");
    assert!(!plain.contains(&secret));

    let outside = tempfile::tempdir().unwrap();
    let refused = sandbox.rullst(outside.path(), &["ai", "review"]);
    if !refused.status.success() {
        assert!(
            text(&refused).contains("git work tree"),
            "{}",
            text(&refused)
        );
    }
}

/// Serves one error context on loopback and returns the base URL.
fn error_server() -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        let body = serde_json::json!({
            "schema": "rullst.error-context.v1", "id": ID,
            "message": "index out of bounds: the len is 0", "file": "src/main.rs", "line": 1,
            "backtrace": ["app::main at ./src/main.rs:1:1"], "method": "GET", "path": "/posts",
            "expires_in_seconds": 1500
        })
        .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8_lossy(&request).into_owned()
    });
    (base, handle)
}

#[test]
fn fix_reads_a_loopback_context_and_only_plans_without_a_terminal() {
    let sandbox = Sandbox::new();
    let (base, server) = error_server();
    let output = sandbox.rullst(&sandbox.project, &["ai", "fix", ID, "--url", &base]);
    assert!(output.status.success(), "{}", text(&output));
    let request = server.join().unwrap();
    assert!(request.starts_with(&format!("GET /_rullst/errors/{ID} HTTP/1.1")));
    let output = text(&output);
    assert!(
        output.contains(&format!("Error {ID} · GET /posts")),
        "{output}"
    );
    assert!(output.contains("Plan only"), "{output}");
    assert!(output.contains("- location: `src/main.rs:1`"), "{output}");
    assert_eq!(
        fs::read_to_string(sandbox.project.join("src/main.rs")).unwrap(),
        "fn main() {}\n"
    );
}

#[test]
fn fix_refuses_remote_sources_and_malformed_ids() {
    let sandbox = Sandbox::new();
    for url in ["http://192.0.2.10:3000", "http://example.com:3000"] {
        let output = sandbox.rullst(&sandbox.project, &["ai", "fix", ID, "--url", url]);
        assert!(!output.status.success());
        assert!(text(&output).contains("refusing"), "{}", text(&output));
    }
    let output = sandbox.rullst(&sandbox.project, &["ai", "fix", "../../etc/passwd"]);
    assert!(!output.status.success());
    assert!(
        text(&output).contains("32 hexadecimal"),
        "{}",
        text(&output)
    );
}

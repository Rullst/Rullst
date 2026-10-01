//! `cargo rullst ai` through the real binary: credentials management and the
//! offline assistant with scripted (non-terminal) standard input, where
//! proposed actions must never execute.
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const PROVIDER_VARIABLES: [&str; 7] = [
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "GEMINI_API_KEY",
    "DEEPSEEK_API_KEY",
    "OLLAMA_HOST",
    "RULLST_AI_MODEL",
    "OPENAI_BASE_URL",
];

struct Sandbox {
    _directory: tempfile::TempDir,
    home: PathBuf,
    project: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let home = directory.path().join("home");
        let project = directory.path().join("app");
        fs::create_dir_all(home.join(".config")).unwrap();
        fs::create_dir_all(project.join("src")).unwrap();
        fs::write(
            project.join("Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13.0.0-alpha.1\"\n",
        )
        .unwrap();
        fs::write(project.join("src/main.rs"), "fn main() {}\n").unwrap();
        Self {
            _directory: directory,
            home,
            project,
        }
    }

    fn config(&self) -> PathBuf {
        self.home.join(".config")
    }

    fn credentials(&self) -> PathBuf {
        self.config().join("rullst").join("credentials.toml")
    }

    fn run(&self, cwd: &Path, args: &[&str], stdin: &str, env: &[(&str, &str)]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rullst"));
        command
            .current_dir(cwd)
            .args(args)
            .env("RULLST_UPDATE_CHECK", "0")
            .env("NO_COLOR", "1")
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.config())
            .env("APPDATA", self.config())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for name in PROVIDER_VARIABLES {
            command.env_remove(name);
        }
        for (name, value) in env {
            command.env(name, value);
        }
        let mut child = command.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
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
fn status_reports_the_offline_assistant_when_nothing_is_configured() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&sandbox.project, &["ai", "status", "--json"], "", &[]);
    assert!(output.status.success(), "{}", text(&output));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema"], "rullst.ai-status.v1");
    assert_eq!(report["provider"], serde_json::Value::Null);
    assert_eq!(report["offline_mock"], true);
    assert_eq!(report["credentials_file_present"], false);
}

#[test]
fn connect_stores_a_private_file_and_environment_variables_win() {
    let sandbox = Sandbox::new();
    let secret = "sk-test-0123-never-print";
    let output = sandbox.run(
        &sandbox.home,
        &[
            "ai",
            "connect",
            "--provider",
            "openai",
            "--model",
            "gpt-4o-mini",
            "--api-key-stdin",
        ],
        &format!("{secret}\n"),
        &[],
    );
    assert!(output.status.success(), "{}", text(&output));
    assert!(!text(&output).contains(secret));
    let stored = fs::read_to_string(sandbox.credentials()).unwrap();
    assert!(stored.contains("provider = \"openai\"") && stored.contains("gpt-4o-mini"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(sandbox.credentials())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    let status = sandbox.run(&sandbox.project, &["ai", "status", "--json"], "", &[]);
    assert!(status.status.success(), "{}", text(&status));
    assert!(!text(&status).contains(secret));
    let report: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(report["provider"], "openai");
    assert_eq!(report["model"], "gpt-4o-mini");
    assert_eq!(report["credential_source"], "credentials file");
    assert_eq!(report["offline_mock"], false);
    assert_eq!(report["streaming"], true);

    let status = sandbox.run(
        &sandbox.project,
        &["ai", "status", "--json"],
        "",
        &[("OPENAI_API_KEY", "mock_from_env")],
    );
    let report: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(report["credential_source"], "environment (OPENAI_API_KEY)");
    assert_eq!(report["offline_mock"], true);

    let human = sandbox.run(&sandbox.project, &["ai", "status"], "", &[]);
    assert!(text(&human).contains("credential:  credentials file"));
    assert!(!text(&human).contains(secret));

    let removed = sandbox.run(&sandbox.home, &["ai", "disconnect"], "", &[]);
    assert!(removed.status.success());
    assert!(!sandbox.credentials().exists());
    let again = sandbox.run(&sandbox.home, &["ai", "disconnect"], "", &[]);
    assert!(text(&again).contains("No stored AI credentials"));
}

#[test]
fn connect_refuses_project_locations_and_needs_flags_without_a_terminal() {
    let sandbox = Sandbox::new();
    let inside = sandbox.project.join(".config");
    let inside = inside.to_str().unwrap();
    let output = sandbox.run(
        &sandbox.project,
        &["ai", "connect", "--provider", "gemini", "--api-key-stdin"],
        "g-key\n",
        &[("XDG_CONFIG_HOME", inside), ("APPDATA", inside)],
    );
    assert!(!output.status.success());
    assert!(
        text(&output).contains("inside the current project"),
        "{}",
        text(&output)
    );
    assert!(!sandbox.project.join(".config").exists());

    let output = sandbox.run(&sandbox.home, &["ai", "connect"], "", &[]);
    assert!(!output.status.success());
    assert!(text(&output).contains("needs --provider"));

    let output = sandbox.run(
        &sandbox.home,
        &["ai", "connect", "--provider", "openai", "--api-key-stdin"],
        "two words\n",
        &[],
    );
    assert!(!output.status.success());
    assert!(!sandbox.credentials().exists());
    assert!(!text(&output).contains("two words"));
}

#[test]
fn one_shot_goals_without_a_terminal_print_the_plan_and_execute_nothing() {
    let sandbox = Sandbox::new();
    // Even scripted approvals must not apply anything.
    let output = sandbox.run(
        &sandbox.project,
        &["ai", "add a posts page"],
        "y\na\ny\n",
        &[],
    );
    assert!(output.status.success(), "{}", text(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("offline mock"), "{stdout}");
    assert!(stdout.contains("Plan only"), "{stdout}");
    assert!(stdout.contains("create rullst-ai-demo.md"));
    assert!(stdout.contains("+ Status: reviewed"));
    assert!(!stdout.contains('\x1b'), "NO_COLOR output is plain");
    assert!(!sandbox.project.join("rullst-ai-demo.md").exists());
}

#[test]
fn scripted_chat_turns_are_plan_only_and_end_at_eof() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(
        &sandbox.project,
        &["ai", "--dry-run"],
        "first goal\n/add src/main.rs\nsecond goal\n",
        &[],
    );
    assert!(output.status.success(), "{}", text(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.matches("Plan only (--dry-run)").count(),
        2,
        "{stdout}"
    );
    assert!(stdout.contains("Attached src/main.rs"));
    assert!(!sandbox.project.join("rullst-ai-demo.md").exists());
}

// src/generators/desktop/runner.rs — Omni app runner for desktop, Android, and iOS.

use crate::ui::spinner::with_spinner;
use colored::*;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Stdio;

/// Line the generated Omni runtime prints right before it opens its window.
pub(crate) const LAUNCH_MARKER: &str = "Launching Omni interface...";
/// The marker of shells generated before the Omni rename.
const LEGACY_LAUNCH_MARKER: &str = "Launching Tauri interface...";
/// Output lines held back while the spinner runs; the rest is streamed.
const MAX_HELD_LINES: usize = 200;

struct ChildGuard(std::process::Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[cfg_attr(mutants, mutants::skip)]
pub fn run_omni_app(target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let omni_dir = Path::new("omni-app");
    if !omni_dir.exists() {
        println!(
            "{}",
            "❌ Error: 'omni-app' directory not found. Please run `cargo rullst make:omni` first."
                .red()
        );
        std::process::exit(1);
    }

    let platform = target.unwrap_or("desktop");

    match platform {
        "desktop" => run_desktop(omni_dir),
        "android" | "ios" => run_mobile(platform, omni_dir),
        _ => {
            println!(
                "{}",
                format!(
                    "❌ Error: Unknown platform '{}'. Supported: desktop, android, ios",
                    platform
                )
                .red()
            );
            std::process::exit(1);
        }
    }
}

fn run_desktop(omni_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = std::process::Command::new("cargo")
        .arg("run")
        .arg("-q")
        .current_dir(omni_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
    let (sender, lines) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if sender.send(line).is_err() {
                break;
            }
        }
    });

    let (launched, held) = with_spinner(
        "🚀 Soon the Omni window will automatically open...",
        || wait_for_launch(lines.iter()),
    );
    for line in held {
        println!("{line}");
    }
    if launched {
        println!("{}", "✅ Omni window launched successfully!".green().bold());
    }
    // The shell and the backend it manages keep logging for their lifetime.
    for line in lines.iter() {
        println!("{line}");
    }
    let _ = reader.join();

    let status = child.wait()?;
    if !status.success() {
        std::process::exit(1);
    }
    Ok(())
}

/// Reads output until the launch marker, the end of the output or
/// `MAX_HELD_LINES`, and returns whether the window launched plus the lines
/// read before it, so the spinner never swallows the application's logs.
fn wait_for_launch(lines: impl Iterator<Item = String>) -> (bool, Vec<String>) {
    let mut held = Vec::new();
    for line in lines {
        if line.contains(LAUNCH_MARKER) || line.contains(LEGACY_LAUNCH_MARKER) {
            return (true, held);
        }
        held.push(line);
        if held.len() >= MAX_HELD_LINES {
            break;
        }
    }
    (false, held)
}

fn run_mobile(platform: &str, omni_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Resolve the CLI before starting anything: a missing CLI must not leave a
    // backend running on :3000 (`std::process::exit` would skip `ChildGuard`).
    let mut tauri_cmd = get_tauri_command(omni_dir)
        .map_err(|error| format!("Omni CLI is required for the {platform} target: {error}"))?;

    println!("🚀 Starting Rullst backend server in background...");
    let backend = std::process::Command::new("cargo")
        .arg("run")
        .arg("-q")
        .current_dir(".")
        .spawn()?;
    let backend_guard = ChildGuard(backend);

    println!("⏳ Waiting for backend to bind...");
    for _ in 0..60 {
        if std::net::TcpStream::connect("127.0.0.1:3000").is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    println!(
        "📱 Starting Omni mobile client ({}) via Omni Engine...",
        platform
    );

    if platform == "android" {
        println!(
            "🔗 Setting up Android USB/Emulator port forwarding (adb reverse tcp:3000 tcp:3000)..."
        );
        let adb_cmd = if cfg!(windows) {
            if let Ok(android_home) = std::env::var("ANDROID_HOME") {
                format!("{}\\platform-tools\\adb.exe", android_home)
            } else {
                "adb".to_string()
            }
        } else {
            "adb".to_string()
        };

        let _ = std::process::Command::new(&adb_cmd)
            .args(["reverse", "tcp:3000", "tcp:3000"])
            .status()
            .or_else(|_| {
                std::process::Command::new("adb")
                    .args(["reverse", "tcp:3000", "tcp:3000"])
                    .status()
            });
    }

    tauri_cmd.arg(platform).arg("dev").current_dir(omni_dir);
    // An error returned here drops the guard, which stops the backend.
    let status = tauri_cmd.status()?;
    drop(backend_guard);
    if !status.success() {
        std::process::exit(1);
    }
    Ok(())
}

pub fn get_tauri_command(
    omni_dir: &Path,
) -> Result<std::process::Command, Box<dyn std::error::Error>> {
    let local_cli = omni_dir
        .join("node_modules")
        .join("@tauri-apps")
        .join("cli")
        .join("package.json");
    if local_cli.is_file() {
        let mut command = if cfg!(windows) {
            std::process::Command::new("npm.cmd")
        } else {
            std::process::Command::new("npm")
        };
        command.args(["exec", "--offline", "--", "tauri"]);
        return Ok(command);
    }

    let has_tauri_cli = std::process::Command::new("cargo")
        .arg("tauri")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_tauri_cli {
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("tauri");
        return Ok(cmd);
    }

    Err(
        "Tauri CLI is unavailable; run `npm install` in omni-app or install a reviewed cargo-tauri version"
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(source: &[&str]) -> impl Iterator<Item = String> {
        source
            .iter()
            .map(|line| (*line).to_string())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn launch_detection_stops_at_the_marker_and_keeps_earlier_output() {
        let mut output = lines(&["backend: serving on 3000", LAUNCH_MARKER, "after launch"]);
        let (launched, held) = wait_for_launch(&mut output);
        assert!(launched);
        assert_eq!(held, ["backend: serving on 3000"]);
        // The rest is left for the caller to stream.
        assert_eq!(output.collect::<Vec<_>>(), ["after launch"]);

        assert!(wait_for_launch(lines(&[LEGACY_LAUNCH_MARKER])).0);
    }

    #[test]
    fn launch_detection_without_a_marker_is_bounded() {
        let (launched, held) = wait_for_launch(lines(&["only", "logs"]));
        assert!(!launched);
        assert_eq!(held, ["only", "logs"]);

        let mut endless = std::iter::repeat_with(|| "log".to_string());
        let (launched, held) = wait_for_launch(&mut endless);
        assert!(!launched);
        assert_eq!(held.len(), MAX_HELD_LINES);
    }
}

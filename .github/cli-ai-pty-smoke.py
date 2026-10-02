#!/usr/bin/env python3
"""`cargo rullst ai` in a real terminal with the offline assistant: review
prompts, the git checkpoint, applied edits, a new project created outside one,
Ctrl+C and Ctrl+D handling, an upgrade finding fixed and checked, and the
plan-only fallback under CI. Local fixtures only: no network or credentials."""
import errno
import json
import os
from pathlib import Path
import pty
import re
import select
import signal
import subprocess
import sys
import tempfile
import time

cli = str(Path(sys.argv[1]).resolve(strict=True))
ansi = re.compile(rb"\x1b\[[0-?]*[ -/]*[@-~]")
prompt = "› ".encode()


def terminal(cwd, env, args, answers, timeout=60):
    """Runs the CLI in a PTY; `answers` are ((marker, occurrence), bytes)."""
    child, fd = pty.fork()
    if child == 0:
        os.chdir(cwd)
        os.execve(cli, [cli, "rullst", *args], env)
    output = bytearray()
    deadline = time.monotonic() + timeout
    try:
        while time.monotonic() < deadline:
            if select.select([fd], [], [], 0.05)[0]:
                try:
                    chunk = os.read(fd, 65536)
                except OSError as error:
                    if error.errno != errno.EIO:
                        raise
                    break
                if not chunk:
                    break
                output.extend(chunk)
            if answers:
                (marker, occurrence), reply = answers[0]
                if ansi.sub(b"", bytes(output)).count(marker) >= occurrence:
                    time.sleep(0.1)
                    os.write(fd, reply)
                    answers.pop(0)
        else:
            raise AssertionError(f"ai session timed out: {bytes(output)[-1200:]!r}")
        _, status = os.waitpid(child, 0)
        child = None
        raw = bytes(output)
        return os.waitstatus_to_exitcode(status), ansi.sub(b"", raw).decode(), raw
    finally:
        os.close(fd)
        if child is not None:
            os.kill(child, signal.SIGKILL)
            os.waitpid(child, 0)


def git(cwd, *args):
    return subprocess.run(["git", "-C", str(cwd), *args], check=True,
                          capture_output=True, text=True).stdout


def project(base, name):
    app = base / name
    (app / "src").mkdir(parents=True)
    (app / "Cargo.toml").write_text(
        f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2024"\n'
        '[dependencies]\nrullst = "13.0.0-alpha.1"\n')
    (app / "src" / "main.rs").write_text("fn main() {}\n")
    git(app, "init", "--quiet")
    git(app, "add", ".")
    git(app, "-c", "user.name=Smoke", "-c", "user.email=smoke@example.invalid",
        "commit", "--quiet", "-m", "initial")
    return app


with tempfile.TemporaryDirectory(prefix="rullst-ai-") as directory:
    base = Path(directory)
    env = {name: value for name, value in os.environ.items()
           if name not in ("CI", "NO_COLOR", "OPENAI_API_KEY", "ANTHROPIC_API_KEY",
                           "GEMINI_API_KEY", "DEEPSEEK_API_KEY", "OLLAMA_HOST",
                           "RULLST_AI_BASE_URL", "RULLST_AI_MODEL", "OPENAI_BASE_URL",
                           "RULLST_ENV", "APP_ENV")}
    env.update(XDG_CONFIG_HOME=str(base / "config"), TERM="xterm-256color",
               RULLST_UPDATE_CHECK="0", CARGO_NET_OFFLINE="true")

    app = project(base, "apply-app")
    answers = [((b"Apply?", 1), b"y\r"), ((b"Apply?", 2), b"y\r"),
               ((b"Run `cargo check` now?", 1), b"n\r")]
    status, text, raw = terminal(app, env, ["ai", "write a demo note"], answers)
    assert status == 0, (status, text[-800:])
    assert (app / "rullst-ai-demo.md").read_text().endswith("Status: reviewed\n"), text
    assert "Checkpoint refs/rullst/ai-checkpoints/" in text and "restore: git restore" in text
    refs = git(app, "for-each-ref", "--format=%(refname)", "refs/rullst/ai-checkpoints")
    assert len(refs.split()) == 1, refs
    assert b"\x1b[32m+ Status: reviewed" in raw, "coloured diff"
    print(json.dumps({"case": "review-and-apply", "passed": True}))

    app = project(base, "interrupt-app")
    answers = [((prompt, 1), b"make a demo\r"), ((b"Apply?", 1), b"\x03"),
               ((prompt, 2), b"\x03"), ((b"(Ctrl+D or /exit quits)", 1), b"\x04")]
    status, text, _ = terminal(app, env, ["ai"], answers)
    assert status == 0, (status, text[-800:])
    assert text.count("Apply?") == 1, "Ctrl+C stopped the turn"
    assert not (app / "rullst-ai-demo.md").exists()
    assert git(app, "for-each-ref", "refs/rullst") == "", "no change, no checkpoint"
    print(json.dumps({"case": "ctrl-c-and-ctrl-d", "passed": True}))

    workspace = base / "workspace"
    workspace.mkdir()
    answers = [((b"Apply?", 1), b"y\r"), ((b"Apply?", 2), b"y\r"),
               ((b"Continue without a checkpoint?", 1), b"y\r"), ((b"Apply?", 3), b"y\r"),
               ((b"Run `cargo check` now?", 1), b"n\r")]
    status, text, _ = terminal(workspace, env, ["ai", "build a shop"], answers, timeout=180)
    assert status == 0, (status, text[-800:])
    created = workspace / "rullst-ai-demo"
    assert (created / "Cargo.toml").is_file(), text[-800:]
    assert "Now working in the new project rullst-ai-demo" in text
    assert (created / "rullst-ai-demo.md").read_text().endswith("Status: reviewed\n")
    print(json.dumps({"case": "new-project-then-change", "passed": True}))

    framework = base / "framework"
    (framework / "src").mkdir(parents=True)
    (framework / "Cargo.toml").write_text(
        '[package]\nname = "rullst"\nversion = "12.1.2"\nedition = "2024"\n')
    (framework / "src" / "lib.rs").write_text(
        "pub mod htmx {\n    pub struct HtmxRequest;\n"
        "    pub fn render_page(_: &HtmxRequest, title: &str, body: String) -> String {\n"
        "        format!(\"{title}{body}\")\n    }\n"
        "    pub fn render_page_with_lang(_: &HtmxRequest, lang: &str, title: &str, body: String)"
        " -> String {\n        format!(\"{lang}{title}{body}\")\n    }\n}\n")
    app = base / "upgrade-app"
    (app / "src").mkdir(parents=True)
    (app / "Cargo.toml").write_text(
        '[package]\nname = "upgrade-app"\nversion = "0.1.0"\nedition = "2024"\n'
        f'[dependencies]\nrullst = {{ version = "12.1.2", path = {json.dumps(str(framework))} }}\n')
    (app / "src" / "main.rs").write_text(
        # Path-qualified so the offline fix leaves no unused import under
        # CI's `-D warnings` (the mock rewrites one call, not the use list).
        "use rullst::htmx::HtmxRequest;\n\n"
        "fn home(htmx: &HtmxRequest) -> String {\n"
        "    rullst::htmx::render_page(htmx, \"Home\", String::new())\n}\n\n"
        "fn main() {\n    println!(\"{}\", home(&HtmxRequest));\n}\n")
    git(app, "init", "--quiet")
    git(app, "add", ".")
    git(app, "-c", "user.name=Smoke", "-c", "user.email=smoke@example.invalid",
        "commit", "--quiet", "-m", "initial")
    answers = [((b"Apply?", 1), b"y\r"), ((b"Apply?", 2), b"y\r")]
    status, text, _ = terminal(app, env, ["ai", "upgrade"], answers, timeout=300)
    assert status == 0, (status, text[-800:])
    assert "REVIEW src/main.rs:4 [V13-RENDER-PAGE-LANGUAGE]" in text, text[-1200:]
    main = (app / "src" / "main.rs").read_text()
    assert 'rullst::htmx::render_page_with_lang(htmx, "en", "Home", String::new())' in main, main
    assert "Checkpoint refs/rullst/ai-checkpoints/" in text
    assert "$ cargo check" in text and "✓ exit status 0" in text, text[-1200:]
    assert "Run `cargo check` now?" not in text, "the proposed check already ran"
    print(json.dumps({"case": "upgrade-fix-and-check", "passed": True}))

    app = project(base, "ci-app")
    status, text, raw = terminal(app, dict(env, CI="true"), ["ai", "write a demo note"], [])
    assert status == 0 and "Plan only: not an interactive terminal" in text, text[-800:]
    assert b"\x1b" not in raw and "Apply?" not in text
    assert not (app / "rullst-ai-demo.md").exists()
    print(json.dumps({"case": "ci-plan-only", "passed": True}))

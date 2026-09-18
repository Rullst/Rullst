#!/usr/bin/env python3
"""Actual terminal/project journey; local stub dependency, no release trust claims."""
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

cli = Path(sys.argv[1]).resolve(strict=True)
version = subprocess.check_output([str(cli), "--version"], text=True).split()[-1]
prompts = [
    b"Copy the selected Git project's working files",
    b"Execute these Cargo commands for this trusted project?",
    b"Approve this complete dependency diff",
]


def terminal(arguments, env, answers):
    child, fd = pty.fork()
    if child == 0:
        os.execve(str(cli), [str(cli), *arguments], env)
    output = bytearray()
    answered = 0
    deadline = time.monotonic() + 90
    try:
        while time.monotonic() < deadline:
            if select.select([fd], [], [], 0.1)[0]:
                try:
                    chunk = os.read(fd, 65536)
                except OSError as error:
                    if error.errno != errno.EIO:
                        raise
                    break
                if not chunk:
                    break
                output.extend(chunk)
                if len(output) > 2 * 1024 * 1024:
                    raise AssertionError("terminal output exceeded 2 MiB")
                if answered < len(answers) and prompts[answered] in output:
                    # Dialoguer accepts a single key. Sending a newline as well
                    # could inadvertently answer the next default-no prompt.
                    os.write(fd, answers[answered])
                    answered += 1
        else:
            raise AssertionError("guided terminal timed out")
        _, status = os.waitpid(child, 0)
        child = None
        return os.waitstatus_to_exitcode(status), bytes(output), answered
    finally:
        os.close(fd)
        if child is not None:
            os.kill(child, signal.SIGKILL)
            os.waitpid(child, 0)


def objects(output):
    text = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", output).decode()
    decoder = json.JSONDecoder()
    cursor = 0
    while (start := text.find("{", cursor)) >= 0:
        try:
            value, consumed = decoder.raw_decode(text[start:])
        except ValueError:
            cursor = start + 1
            continue
        yield value
        cursor = start + consumed


for name, answers in [
    ("decline-copy", [b"\r"]),
    ("decline-execution", [b"y", b"\r"]),
    ("apply-and-recover", [b"y", b"y", b"y"]),
    ("failing-project", [b"y", b"y"]),
]:
    with tempfile.TemporaryDirectory(prefix="rullst-guided-") as directory:
        base = Path(directory)
        app = base / "app with spaces"
        (app / "src").mkdir(parents=True)
        (app / "vendor/framework/src").mkdir(parents=True)
        (app / "Cargo.toml").write_text(
            '[package]\nname="guided-app"\nversion="0.1.0"\nedition="2024"\n'
            f'[dependencies]\nrullst-core={{version="{version.split(".")[0]}",path="vendor/framework"}}\n'
        )
        (app / "vendor/framework/Cargo.toml").write_text(
            f'[package]\nname="rullst-core"\nversion="{version}"\nedition="2024"\n'
        )
        (app / "vendor/framework/src/lib.rs").write_text("pub fn marker() {}\n")
        body = "fn main() {}\n#[test] fn acceptance() { assert!(true); }\n"
        if name == "failing-project":
            body = body.replace("assert!(true)", 'panic!("application fixture failed")')
        (app / "src/main.rs").write_text(body)
        (app / "build.rs").write_text(
            'fn main() { std::fs::write(std::env::var("GUIDED_BUILD_MARKER").unwrap(), b"ran").unwrap(); }'
        )
        (app / ".gitignore").write_text("target/\nCargo.lock\n.env\n")
        subprocess.run(["git", "init", "--quiet", str(app)], check=True)
        subprocess.run(["git", "-C", str(app), "add", "."], check=True)
        env = dict(os.environ, XDG_CACHE_HOME=str(base), LOCALAPPDATA=str(base),
                   CARGO_NET_OFFLINE="true", RULLST_DISABLE_UPDATE_CHECK="true",
                   GUIDED_BUILD_MARKER=str(base / "executed"), TERM="xterm")
        subprocess.run(["cargo", "generate-lockfile", "--offline"], cwd=app,
                       env=env, check=True, capture_output=True)
        original = (app / "Cargo.toml").read_bytes()
        lock = (app / "Cargo.lock").read_bytes()
        status, output, answered = terminal(
            ["update", "guided", "--to", version, "--scope", "project",
             "--project", str(app), "--offline", "--timeout-seconds", "30"], env, answers)
        assert answered == len(answers), output.decode()
        assert (status == 0) == (name != "failing-project"), output.decode()
        assert not (app / "target").exists()
        assert (app / "src/main.rs").read_text() == body
        reports = list(objects(output))
        if name == "apply-and-recover":
            assert f'version="={version}"'.encode() in (app / "Cargo.toml").read_bytes()
            recovery = next(item["recovery_args"] for item in reports if "recovery_args" in item)
            subprocess.run([str(cli), "update", *recovery, "--json"], env=env,
                           check=True, capture_output=True)
        assert (app / "Cargo.toml").read_bytes() == original
        assert (app / "Cargo.lock").read_bytes() == lock
        assert (base / "executed").exists() == (name in ("apply-and-recover", "failing-project"))
        timings = [item["elapsed_ms"] for item in reports if "elapsed_ms" in item]
        print(json.dumps({"case": name, "passed": True, "stage_elapsed_ms": timings}))

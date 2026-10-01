#!/usr/bin/env python3
"""No-argument home screen in a real terminal: opening, daily motion, key skip,
terminal restoration, opt-outs and the project summary. Local fixtures only."""
import datetime
import errno
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time

cli = str(Path(sys.argv[1]).resolve(strict=True))
version = subprocess.check_output([cli, "--version"], text=True).split()[-1]
plain = f"RULLST v{version} · SECURE, FAST AND AI-NATIVE RUST FRAMEWORK".encode()
menu = b"Navigate with"
redraw = b"\x1b[8A"
ansi = re.compile(rb"\x1b\[[0-?]*[ -/]*[@-~]")


def terminal(cwd, env, answers):
    """Runs the CLI in a PTY; `answers` are (condition, bytes) pairs."""
    child, fd = pty.fork()
    if child == 0:
        os.chdir(cwd)
        os.execve(cli, [cli], env)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
    output = bytearray()
    first_frame = None
    restored = None
    deadline = time.monotonic() + 30
    try:
        while time.monotonic() < deadline:
            if select.select([fd], [], [], 0.01)[0]:
                try:
                    chunk = os.read(fd, 65536)
                except OSError as error:
                    if error.errno != errno.EIO:
                        raise
                    break
                if not chunk:
                    break
                output.extend(chunk)
            if first_frame is None and "╗".encode() in output:
                first_frame = time.monotonic()
            if answers and answers[0][0](output, first_frame):
                os.write(fd, answers.pop(0)[1])
        else:
            raise AssertionError(f"home screen timed out: {bytes(output)[-800:]!r}")
        lflag = termios.tcgetattr(fd)[3]
        restored = bool(lflag & termios.ICANON) and bool(lflag & termios.ECHO)
        _, status = os.waitpid(child, 0)
        child = None
        return os.waitstatus_to_exitcode(status), bytes(output), restored
    finally:
        os.close(fd)
        if child is not None:
            os.kill(child, signal.SIGKILL)
            os.waitpid(child, 0)


def at_menu(output, _first_frame):
    return menu in output


def after_first_frame(output, first_frame):
    return first_frame is not None


exit_menu = [(at_menu, b"\x1b[A"), (at_menu, b"\r")]

with tempfile.TemporaryDirectory(prefix="rullst-home-") as directory:
    base = Path(directory)
    cache = base / "cache"
    stamp = cache / "rullst-ui-v1" / "last-opening"
    outside = base / "outside"
    outside.mkdir()
    app = base / "smoke-app"
    nested = app / "src" / "controllers"
    nested.mkdir(parents=True)
    (app / "src" / "migrations").mkdir()
    for name in ["mod.rs", "m20260101000000_create_users.rs", "m20260915120000_create_items.rs"]:
        (app / "src" / "migrations" / name).write_text("")
    (app / "Cargo.toml").write_text(
        '[package]\nname = "smoke-app"\nversion = "0.1.0"\nedition = "2024"\n'
        '[dependencies]\nrullst = { version = "13", default-features = false, '
        'features = ["orm", "studio"] }\n'
    )
    (app / ".env").write_text("DATABASE_URL=sqlite://smoke.db?mode=rwc&password=hunter2\n")
    subprocess.run(["git", "init", "--quiet", str(app)], check=True)
    subprocess.run(["git", "-C", str(app), "symbolic-ref", "HEAD", "refs/heads/feat/home"],
                   check=True)

    env = {name: value for name, value in os.environ.items()
           if name not in ("CI", "NO_COLOR", "RULLST_REDUCED_MOTION", "COLORTERM", "DATABASE_URL",
                           "TURSO_DATABASE_URL")}
    env.update(XDG_CACHE_HOME=str(cache), LOCALAPPDATA=str(cache), TERM="xterm-256color",
               RULLST_DISABLE_UPDATE_CHECK="true", CARGO_NET_OFFLINE="true")
    today = datetime.date.today().isoformat()

    def case(name, cwd, extra, answers, check):
        status, output, restored = terminal(cwd, dict(env, **extra), list(answers))
        frames = output.count(redraw)
        check(status, output, frames, restored)
        print(json.dumps({"case": name, "passed": True, "status": status, "redraws": frames}))

    def first_run(status, output, frames, restored):
        assert status == 0 and frames == 42 and restored, (status, frames, restored)
        assert b"38;2;40;120;255m" in output, "truecolor gradient"
        assert stamp.read_text().strip() == today
        text = ansi.sub(b"", output).decode()
        for expected in ["Project     smoke-app", "Root        ../..", "Features    orm · studio",
                         "Database    SQLite · .env", "Git branch  feat/home",
                         "Migrations  2 defined · latest m20260915120000_create_items"]:
            assert expected in text, expected
        assert "hunter2" not in text and "Start Dev Server" in text

    case("first-run-of-the-day", nested, {"COLORTERM": "truecolor"}, exit_menu, first_run)

    def static(status, output, frames, restored):
        assert status == 0 and frames == 0 and restored and menu in output, (status, frames)

    case("later-run-static", nested, {"COLORTERM": "truecolor"}, exit_menu, static)

    stamp.unlink()

    def skipped(status, output, frames, restored):
        assert status == 0 and 0 < frames < 42 and restored, (status, frames)
        assert b"38;5;33m" in output and b"38;2;" not in output, "xterm-256 fallback"
        assert "Create New Project" in ansi.sub(b"", output).decode()

    case("any-key-skips", outside, {}, [(after_first_frame, b" ")] + exit_menu, skipped)

    stamp.unlink()

    def interrupted(status, output, frames, restored):
        assert status == 1 and b"read interrupted" in output and restored, (status, restored)
        assert output.count(b"\x1b[?25h") >= 1, "cursor shown again"

    case("ctrl-c-restores-terminal", outside, {"COLORTERM": "truecolor"},
         [(after_first_frame, b"\x03")], interrupted)

    stamp.unlink()

    def reduced(status, output, frames, restored):
        assert status == 0 and frames == 0 and not stamp.exists() and b"38;2;" in output

    case("reduced-motion", outside, {"RULLST_REDUCED_MOTION": "1", "COLORTERM": "truecolor"},
         exit_menu, reduced)

    def no_color(status, output, frames, restored):
        assert status == 0 and frames == 0 and plain in output and menu in output
        assert b"38;2;" not in output and b"38;5;" not in output

    case("no-color", outside, {"NO_COLOR": "1"}, exit_menu, no_color)

    for name, extra in [("term-dumb", {"TERM": "dumb"}), ("ci", {"CI": "true"})]:
        def automation(status, output, frames, restored):
            assert status == 0 and output.startswith(plain) and menu not in output
            assert b"\x1b" not in output and b"cargo rullst new <name>" in output

        case(name, outside, extra, [], automation)

    piped = subprocess.run([cli], cwd=nested, env=env, stdin=subprocess.DEVNULL,
                           capture_output=True, timeout=30)
    assert piped.returncode == 0 and piped.stdout.startswith(plain + b"\n\n"), piped
    assert b"Next steps (from ../..)" in piped.stdout and b"\x1b" not in piped.stdout
    print(json.dumps({"case": "non-tty", "passed": True, "status": piped.returncode}))

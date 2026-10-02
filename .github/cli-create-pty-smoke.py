#!/usr/bin/env python3
"""`cargo rullst new` wizard and `cargo rullst tour` in a real terminal: answers,
back/cancel, Ctrl+C, terminal restoration and colour opt-outs. Nothing is built:
every run skips the initial migration. Local files only."""
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
import sys
import tempfile
import termios
import time

cli = str(Path(sys.argv[1]).resolve(strict=True))
ansi = re.compile(rb"\x1b\[[0-?]*[ -/]*[@-~]")
DOWN, ENTER, SPACE, ESC, CTRL_C = b"\x1b[B", b"\r", b" ", b"\x1b", b"\x03"


def terminal(arguments, cwd, env, answers):
    """Runs the CLI in a PTY; each answer waits for its marker after the last."""
    child, fd = pty.fork()
    if child == 0:
        os.chdir(cwd)
        os.execve(cli, [cli, *arguments], env)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 110, 0, 0))
    output = bytearray()
    answered = 0
    since = 0
    deadline = time.monotonic() + 60
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
                continue
            if answered < len(answers) and answers[answered][0] in output[since:]:
                since = output.rfind(answers[answered][0]) + 1
                # A lone ESC is only a key once the terminal has gone quiet.
                os.write(fd, answers[answered][1])
                answered += 1
        else:
            raise AssertionError(f"terminal timed out: {bytes(output)[-1500:]!r}")
        lflag = termios.tcgetattr(fd)[3]
        restored = bool(lflag & termios.ICANON) and bool(lflag & termios.ECHO)
        _, status = os.waitpid(child, 0)
        child = None
        assert answered == len(answers), (answered, bytes(output)[-1500:])
        return os.waitstatus_to_exitcode(status), bytes(output), restored
    finally:
        os.close(fd)
        if child is not None:
            os.kill(child, signal.SIGKILL)
            os.waitpid(child, 0)


def plain(output):
    return ansi.sub(b"", output).decode("utf-8", "replace")


with tempfile.TemporaryDirectory(prefix="rullst-create-") as directory:
    base = Path(directory)
    env = {name: value for name, value in os.environ.items()
           if name not in ("CI", "NO_COLOR", "RULLST_REDUCED_MOTION", "COLORTERM", "PORT")}
    env.update(XDG_CACHE_HOME=str(base / "cache"), TERM="xterm-256color",
               COLORTERM="truecolor", RULLST_DISABLE_UPDATE_CHECK="true",
               CARGO_NET_OFFLINE="true")

    def case(name, arguments, answers, check, extra=None):
        cwd = base / name
        cwd.mkdir()
        status, output, restored = terminal(arguments, cwd, dict(env, **(extra or {})), answers)
        check(cwd, status, output, restored)
        print(json.dumps({"case": name, "passed": True, "status": status}))

    def created(cwd, status, output, restored):
        text = plain(output)
        assert status == 0 and restored, (status, restored, text[-1500:])
        for expected in ["Step 2 of 6 · Blueprint", "Step 6 of 6 · Review",
                         "files will be written", "✔ Features · AI features",
                         "Next steps", "open http://127.0.0.1:3000"]:
            assert expected in text, expected
        assert b"38;2;" in output, "truecolor screens"
        manifest = (cwd / "smoke_app" / "Cargo.toml").read_text()
        assert '"ai"' in manifest, manifest

    case("wizard-create", ["new", "--skip-initial-migration"], [
        (b"Project name", b"smoke_app\r"),
        (b"Which starter", ENTER),
        (b"What are you building", ENTER),
        (b"Which primary database", ENTER),
        (b"Optional features", SPACE),
        (b"Optional features", ENTER),
        (b"Ready?", ENTER),
    ], created)

    def cancelled(cwd, status, output, restored):
        text = plain(output)
        assert status == 0 and restored, (status, restored, text[-1500:])
        assert "Step 2 of 4 · Database" in text and "Cancelled. Nothing was created." in text
        assert list(cwd.iterdir()) == [], "cancel writes nothing"

    case("wizard-back-and-cancel", ["new", "later", "--skip-initial-migration"], [
        (b"Which starter", DOWN),
        (b"Which starter", ENTER),
        (b"Which primary database", ESC),
        (b"Which starter", ENTER),
        (b"Which primary database", ENTER),
        (b"Optional features", ENTER),
        (b"Ready?", DOWN + DOWN),
        (b"Ready?", ENTER),
    ], cancelled)

    def interrupted(cwd, status, output, restored):
        assert status == 1 and restored and b"read interrupted" in output, (status, restored)
        assert output.count(b"\x1b[?25h") >= 1, "cursor shown again"
        assert list(cwd.iterdir()) == []

    case("wizard-ctrl-c", ["new", "stopped"], [(b"Which starter", CTRL_C)], interrupted)

    def toured(cwd, status, output, restored):
        text = plain(output)
        assert status == 0 and restored, (status, restored, text[-1500:])
        for expected in ["Step 1 of 7 · new", "Dry run: nothing was created.",
                         "finished (exit status 0)", "Step 2 of 7 · dev", "Tour closed."]:
            assert expected in text, expected
        assert b"\x1b[38;" not in output, "NO_COLOR keeps screens plain"
        assert list(cwd.iterdir()) == [], "the tour never writes"

    case("tour-example-and-quit", ["tour"], [
        (b"Step 1 of 7", DOWN),
        (b"Step 1 of 7", ENTER),
        (b"finished (exit status", ENTER),
        (b"Step 2 of 7", DOWN + DOWN + DOWN),
        (b"Step 2 of 7", ENTER),
    ], toured, {"NO_COLOR": "1"})

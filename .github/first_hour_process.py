"""Processes of the first-hour check: commands, servers and the wizard's terminal."""

from __future__ import annotations

import contextlib
from dataclasses import dataclass
import errno
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import shlex
import signal
import socket
import struct
import subprocess
import sys
import termios
import threading
import time

import first_hour_checks as checks
from first_hour_checks import Failure, log

ANSI = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]|\x1b[()][A-Z0-9]|\r")
BUILD_DEADLINE = 25 * 60


@dataclass
class Result:
    returncode: int
    output: str


def port_open(port: int = checks.PORT) -> bool:
    with socket.socket() as probe:
        probe.settimeout(1)
        return probe.connect_ex(("127.0.0.1", port)) == 0


class Server:
    """A server the reader starts in a second terminal and stops with Ctrl+C."""

    def __init__(self, tokens: list[str], cwd: Path, env: dict[str, str], log_path: Path):
        if port_open():
            raise Failure("port 3000 is already in use before the server starts")
        self.command = shlex.join(tokens)
        self.log_path = log_path
        self.log_file = open(log_path, "w")
        log(f"$ {self.command} &   # output: {log_path.name}")
        self.process = subprocess.Popen(tokens, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                        stdout=self.log_file, stderr=subprocess.STDOUT,
                                        start_new_session=True)

    def text(self) -> str:
        return self.log_path.read_text(errors="replace")

    def tail(self, lines: int = 150) -> str:
        return "\n".join(ANSI.sub("", self.text()).splitlines()[-lines:])

    def wait_until_ready(self) -> None:
        started = time.monotonic()
        while time.monotonic() - started < BUILD_DEADLINE:
            if self.process.poll() is not None:
                raise Failure(f"`{self.command}` exited with {self.process.returncode} "
                              f"before port 3000 answered:\n{self.tail()}")
            with contextlib.suppress(Failure):
                checks.get("/")
                log(f"ok: `{self.command}` answers on port 3000 after {time.monotonic() - started:.0f}s")
                return
            time.sleep(2)
        raise Failure(f"`{self.command}` did not answer on port 3000 within "
                      f"{BUILD_DEADLINE // 60} minutes:\n{self.tail()}")

    def stop(self) -> None:
        """Ctrl+C: SIGINT to the terminal's foreground process group."""
        try:
            if self.process.poll() is None:
                os.killpg(self.process.pid, signal.SIGINT)
                try:
                    self.process.wait(timeout=60)
                except subprocess.TimeoutExpired:
                    os.killpg(self.process.pid, signal.SIGKILL)
                    self.process.wait(timeout=30)
                    raise Failure(f"`{self.command}` did not stop within 60s of Ctrl+C:\n{self.tail()}")
            for _ in range(30):
                if not port_open():
                    log(f"ok: Ctrl+C stopped `{self.command}` and freed port 3000")
                    return
                time.sleep(1)
            raise Failure(f"port 3000 still answers after `{self.command}` stopped:\n{self.tail()}")
        finally:
            self.log_file.close()


def stream(tokens: list[str], cwd: Path, env: dict[str, str], *, check: bool = True,
           timeout: int = BUILD_DEADLINE) -> Result:
    """Runs one command, streaming its output to the log and keeping a copy."""
    log(f"$ {shlex.join(tokens)}")
    process = subprocess.Popen(tokens, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                               errors="replace", start_new_session=True)
    captured: list[str] = []

    def pump() -> None:
        assert process.stdout is not None
        for line in process.stdout:
            captured.append(line)
            sys.stdout.write(line)
        sys.stdout.flush()

    reader = threading.Thread(target=pump, daemon=True)
    reader.start()
    try:
        process.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()
        raise Failure(f"`{shlex.join(tokens)}` did not finish within {timeout // 60} minutes")
    reader.join(timeout=30)
    result = Result(process.returncode, ANSI.sub("", "".join(captured)))
    if check and result.returncode != 0:
        raise Failure(f"`{shlex.join(tokens)}` exited with {result.returncode}")
    return result


def terminal(tokens: list[str], cwd: Path, env: dict[str, str], prompts: tuple[bytes, ...], *,
             check: bool = True) -> Result:
    """Runs a command in a pseudo-terminal, pressing Enter at each prompt in turn."""
    log(f"$ {shlex.join(tokens)}   # in a terminal; Enter on each question")
    env = {k: v for k, v in env.items() if k not in ("CI", "NO_COLOR")}
    env["TERM"] = "xterm-256color"
    child, fd = pty.fork()
    if child == 0:  # The child never returns into the caller.
        try:
            os.chdir(cwd)
            os.execvpe(tokens[0], tokens, env)
        finally:
            os._exit(127)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
    output, answered, since = bytearray(), 0, 0
    started = last = time.monotonic()
    try:
        while time.monotonic() - started < BUILD_DEADLINE:
            if answered < len(prompts) and time.monotonic() - last > 120:
                raise Failure(f"the terminal never asked `{prompts[answered].decode()}`:\n"
                              + ANSI.sub("", output.decode(errors="replace"))[-3000:])
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
                continue
            if answered < len(prompts) and prompts[answered] in output[since:]:
                since = output.rfind(prompts[answered]) + 1
                os.write(fd, b"\r")
                answered, last = answered + 1, time.monotonic()
        else:
            raise Failure(f"`{shlex.join(tokens)}` did not finish within {BUILD_DEADLINE // 60} minutes")
        _, status = os.waitpid(child, 0)
        child = 0
    finally:
        os.close(fd)
        if child:
            os.kill(child, signal.SIGKILL)
            os.waitpid(child, 0)
    text = ANSI.sub("", output.decode(errors="replace"))
    log("\n".join(text.splitlines()[-80:]))
    result = Result(os.waitstatus_to_exitcode(status), text)
    if check and result.returncode != 0:
        raise Failure(f"`{shlex.join(tokens)}` exited with {result.returncode}")
    return result

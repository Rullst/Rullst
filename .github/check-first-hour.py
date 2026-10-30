#!/usr/bin/env python3
"""Follow the documentation's first hour as a newcomer would (Linux CI only).

Runs the blocks marked `<!-- first-hour: ... -->` in README.md, the getting
started page and the first tutorials (.github/first_hour_docs.py), in reading
order, in a private scratch directory: commands as written, code applied to
the files the page names. `cargo install cargo-rullst --version =<candidate>`
installs the CLI from the packaged archives, and a `[patch.crates-io]` in the
scratch directory's `.cargo/config.toml` resolves the unpublished release
crates to those archives, so generated and hand-written projects are built
exactly as written. `cargo run` and `cargo rullst dev` run in the background,
like a second terminal; once port 3000 answers, the page's checks run
(.github/first_hour_checks.py), and the next block stops the server as Ctrl+C
would. `cargo rullst new` without `--default` answers the wizard's questions
with Enter in a pseudo-terminal. A failure is annotated on the page's line.

Usage: check-first-hour.py [--packages DIR] [--journey NAME ...] | --list | --pages
"""

from __future__ import annotations

import argparse
import contextlib
from dataclasses import dataclass
import errno
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import shlex
import shutil
import signal
import socket
import struct
import subprocess
import sys
import tarfile
import tempfile
import termios
import threading
import time
import tomllib
import traceback

import first_hour_checks as checks
from first_hour_checks import Failure, expect, log
import first_hour_docs as docs
from first_hour_docs import Block, MarkerError

ROOT = docs.ROOT
TIMINGS: list[tuple[str, str, float]] = []
ANSI = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]|\x1b[()][A-Z0-9]|\r")
# Enter accepts the wizard's suggestion on each screen (Blank, SQLite, none).
WIZARD = (b"Which starter", b"Which primary database", b"Optional features", b"Ready?")
SERVERS = (["cargo", "run"], ["cargo", "rullst", "dev"])
BUILD_DEADLINE = 25 * 60


@dataclass
class Result:
    returncode: int
    output: str


@contextlib.contextmanager
def step(page: str, title: str):
    """Collapses a step's output in the Actions log and records its duration."""
    log(f"::group::{Path(page).name} · {title}")
    started = time.monotonic()
    try:
        yield
    finally:
        TIMINGS.append((page, title, time.monotonic() - started))
        log("::endgroup::")


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
                    raise Failure(f"`{self.command}` did not stop within 60s of Ctrl+C")
            for _ in range(30):
                if not port_open():
                    log(f"ok: Ctrl+C stopped `{self.command}` and freed port 3000")
                    return
                time.sleep(1)
            raise Failure(f"port 3000 still answers after `{self.command}` stopped")
        finally:
            self.log_file.close()


class Session:
    """The reader's terminal: working directory, installed CLI and last output."""

    def __init__(self, work: Path, packages: Path, version: str):
        self.work, self.packages, self.version = work, packages, version
        self.crates = docs.release_crates()
        self.journeys = work / "journeys"
        self.journeys.mkdir()
        self.env = dict(os.environ, RULLST_DISABLE_UPDATE_CHECK="true")
        self.env.pop("PORT", None)
        self.cwd = self.start = self.journeys
        self.output = ""
        self.server: Server | None = None
        self.last_server: Server | None = None
        self.server_count = 0

    # Hook API ---------------------------------------------------------------
    def page_text(self, page: str) -> str:
        return (ROOT / page).read_text(encoding="utf-8")

    def server_log(self) -> str:
        return self.server.text() if self.server else ""

    def run(self, tokens: list[str], *, check: bool = True, timeout: int = BUILD_DEADLINE,
            cwd: Path | None = None) -> Result:
        """Runs one command in the reader's directory, streaming its output."""
        log(f"$ {shlex.join(tokens)}")
        process = subprocess.Popen(tokens, cwd=cwd or self.cwd, env=self.env, stdin=subprocess.DEVNULL,
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

    def run_commands(self, block: Block, *, check: bool = True) -> Result:
        """The block's commands in order, as a reader types them."""
        outputs, code, earlier = [], 0, self.server
        for tokens in docs.commands(block.code):
            if self.server is not earlier:
                raise Failure(f"{block.where}: a command follows the server in the same block")
            result = self.command(tokens, block, check=check)
            outputs.append(result.output)
            code = code or result.returncode
            if result.returncode != 0:
                break
        self.output = "".join(outputs)
        return Result(code, self.output)

    # Commands ---------------------------------------------------------------
    def command(self, tokens: list[str], block: Block, *, check: bool) -> Result:
        docs.check_command(tokens, self.version, self.crates)
        if tokens[0] == "cd" and len(tokens) == 2:
            target = (self.cwd / tokens[1]).resolve()
            if not target.is_dir():
                raise Failure(f"`cd {tokens[1]}`: no such directory")
            self.cwd = target
            log(f"$ cd {tokens[1]}")
            return Result(0, "")
        if tokens in SERVERS:
            self.server_count += 1
            path = self.work / f"server-{self.server_count}-{block.step}.log"
            self.server = Server(tokens, self.cwd, self.env, path)
            self.server.wait_until_ready()
            return Result(0, "")
        if tokens[:3] == ["cargo", "install", docs.CLI]:
            # check_command verified the version; the archives are installed.
            log(f"$ {shlex.join(tokens)}   # installed from the packaged archives")
            return self.run(["cargo", "rullst", "--version"])
        if tokens[:2] == ["cargo", "add"]:
            return self.cargo_add(tokens, check=check)
        if tokens[:3] == ["cargo", "rullst", "new"] and "--default" not in tokens:
            return self.in_terminal(tokens, check=check)
        return self.run(tokens, check=check)

    def install_cli(self) -> None:
        """`cargo install` from the archive, as users get it; it writes registry pins."""
        extracted = self.work / "packages"
        extracted.mkdir()
        for crate in self.crates:
            archive = self.packages / f"{crate}-{self.version}.crate"
            if not archive.is_file():
                raise Failure(f"missing package archive {archive}")
            with tarfile.open(archive) as handle:
                handle.extractall(extracted, filter="data")
        paths = {crate: str(extracted / f"{crate}-{self.version}")
                 for crate in self.crates if crate != docs.CLI}
        patch = "\n".join(["", "# Added by .github/check-first-hour.py: the release crates are not",
                           "# on crates.io before publication; the packaged archives replace them.",
                           "[patch.crates-io]",
                           *(f'"{crate}" = {{ path = {json.dumps(path)} }}' for crate, path in paths.items())])
        manifest = Path(paths["rullst"]).parent / f"{docs.CLI}-{self.version}" / "Cargo.toml"
        manifest.write_text(manifest.read_text() + patch + "\n")
        # Every project of every journey resolves the release crates to the
        # archives through this file; no project file is changed.
        (self.journeys / ".cargo").mkdir()
        (self.journeys / ".cargo/config.toml").write_text(patch.lstrip() + "\n")
        root = self.work / "install"
        # Outside the journeys: the archive's own manifest carries the patch.
        self.run(["cargo", "install", "--path", str(manifest.parent), "--root", str(root),
                  "--offline", "--locked", "--debug", "--force"], timeout=45 * 60, cwd=self.work)
        self.env["PATH"] = f"{root / 'bin'}{os.pathsep}{self.env['PATH']}"
        self.run(["cargo", "rullst", "--version"], cwd=self.work)

    def cargo_add(self, tokens: list[str], *, check: bool) -> Result:
        """`cargo add` as written; an unpublished release crate gets the line it writes."""
        result = self.run(tokens, check=False)
        crate, _, requirement = (tokens[2] if len(tokens) == 3 else "").partition("@")
        if (result.returncode != 0 and crate in self.crates and requirement == f"={self.version}"
                and "could not be found in registry index" in result.output):
            manifest = self.cwd / "Cargo.toml"
            text = manifest.read_text()
            if "\n[dependencies]\n" not in text:
                raise Failure(f"{manifest} has no [dependencies] table")
            manifest.write_text(text.replace("\n[dependencies]\n",
                                             f'\n[dependencies]\n{crate} = "{requirement}"\n', 1))
            log(f"note: {crate} {self.version} is not on crates.io before its release, so the "
                f'check wrote the line `cargo add` writes: {crate} = "{requirement}"')
            return Result(0, result.output)
        if check and result.returncode != 0:
            raise Failure(f"`{shlex.join(tokens)}` exited with {result.returncode}")
        return result

    def in_terminal(self, tokens: list[str], *, check: bool) -> Result:
        """The create wizard in a pseudo-terminal, answering each question with Enter."""
        log(f"$ {shlex.join(tokens)}   # in a terminal; Enter on each question")
        env = {k: v for k, v in self.env.items() if k not in ("CI", "NO_COLOR")}
        env["TERM"] = "xterm-256color"
        child, fd = pty.fork()
        if child == 0:  # The child never returns into this script.
            try:
                os.chdir(self.cwd)
                os.execvpe(tokens[0], tokens, env)
            finally:
                os._exit(127)
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
        output, answered, since = bytearray(), 0, 0
        started = last = time.monotonic()
        try:
            while time.monotonic() - started < BUILD_DEADLINE:
                if answered < len(WIZARD) and time.monotonic() - last > 120:
                    raise Failure(f"the wizard never asked `{WIZARD[answered].decode()}`:\n"
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
                if answered < len(WIZARD) and WIZARD[answered] in output[since:]:
                    since = output.rfind(WIZARD[answered]) + 1
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

    # Blocks -----------------------------------------------------------------
    def stop_server(self) -> None:
        if self.server:
            server = self.last_server = self.server
            self.server = None
            server.stop()

    def execute(self, block: Block) -> None:
        key = (block.page, block.step)
        keeps_server = block.action == "output" or (
            block.language in docs.SHELL
            and all(tokens[0] == "curl" for tokens in docs.commands(block.code)))
        if not keeps_server:
            self.stop_server()  # The reader stops the server before going on.
        if block.options.get("new-terminal"):
            self.cwd = self.start
        if block.language in docs.SHELL:
            (checks.RUN.get(key) or (lambda session, b: session.run_commands(b)))(self, block)
        elif block.action == "apply":
            checks.APPLY[key](self, block)
        elif block.action == "output":
            expect(docs.contains_text(self.output, block.code),
                   f"the previous command printed the documented `{block.language}` output")
        elif block.action == "expect-file":
            path = self.cwd / str(block.options["expect-file"])
            present = {line.strip() for line in path.read_text().splitlines()}
            missing = [line for line in block.code.splitlines() if line.strip() not in present]
            expect(not missing, f"{path.name} contains the documented lines (missing: {missing})")
        else:
            self.edit(block)
        if key in checks.AFTER:
            checks.AFTER[key](self, block)
        if self.server and key in checks.SERVER:
            checks.SERVER[key](self, block)

    def edit(self, block: Block) -> None:
        pattern = str(block.options[block.action])
        matches = sorted(self.cwd.glob(pattern))
        if len(matches) > 1 or (not matches and (block.action != "write" or "*" in pattern)):
            raise Failure(f"`{pattern}` matches {len(matches)} files in {self.cwd.name}")
        path = matches[0] if matches else self.cwd / pattern
        old = path.read_text() if path.exists() else None
        try:
            path.write_text(docs.edit(block, old))
        except MarkerError as error:
            raise Failure(str(error)) from error
        log(f"applied the {block.language} block to {path.relative_to(self.cwd)} ({block.action})")


def remove_build_outputs(directory: Path) -> None:
    """Frees disk between journeys: every `target` directory next to a Cargo.toml."""
    for parent, names, _ in os.walk(directory):
        if "target" in names and (Path(parent) / "Cargo.toml").is_file():
            shutil.rmtree(Path(parent) / "target", ignore_errors=True)
            names.remove("target")
        names[:] = [name for name in names if name not in (".git", "target")]


def annotate(block: Block, error: Exception) -> None:
    message = str(error).replace("%", "%25").replace("\r", "").replace("\n", "%0A")
    log(f"::error file={block.page},line={block.line},title=First-hour documentation "
        f"({block.step})::{message}")


def run_journey(session: Session, name: str, pages: dict[str, list[Block]]) -> bool:
    session.start = session.cwd = session.journeys / name
    session.start.mkdir()
    session.output = ""
    session.last_server = None
    log(f"## Journey `{name}`: {', '.join(docs.JOURNEYS[name])}")
    block: Block | None = None
    try:
        for page in docs.JOURNEYS[name]:
            for block in pages[page]:
                with step(page, block.step):
                    session.execute(block)
            block = None
        session.stop_server()
        return True
    except Exception as error:  # A failed journey must not stop the next ones.
        if not isinstance(error, (Failure, MarkerError)):
            traceback.print_exc()
        server = session.server or session.last_server
        if server:
            log(f"::group::Server log ({server.command})\n{server.tail(300)}\n::endgroup::")
        if block is not None:
            annotate(block, error)
        else:
            log(f"::error title=First-hour documentation ({name})::{error}")
        with contextlib.suppress(Failure, OSError):
            session.stop_server()
        session.server = None
        return False
    finally:
        remove_build_outputs(session.start)
        subprocess.run(["df", "-h", "/"], check=False)


def candidate_version() -> str:
    return tomllib.loads((ROOT / "cargo-rullst/Cargo.toml").read_text())["package"]["version"]


def validate(pages: dict[str, list[Block]], version: str) -> dict[str, list[Block]]:
    """Fails before any build when a documented command or a hook cannot run."""
    crates = docs.release_crates()
    for blocks in pages.values():
        for block in blocks:
            if block.language in docs.SHELL:
                commands = docs.commands(block.code)
                for tokens in commands:
                    docs.check_command(tokens, version, crates)
                if any(tokens in SERVERS for tokens in commands[:-1]):
                    raise MarkerError(f"{block.where}: a server command must end its block")
            if block.action == "apply" and (block.page, block.step) not in checks.APPLY:
                raise MarkerError(f"{block.where}: an `apply` block needs an APPLY hook")
    for kind, hooks in checks.HOOKS.items():
        for page, step_name in hooks:
            if not any(block.step == step_name for block in pages.get(page, [])):
                raise MarkerError(f"the {kind} hook for {page} ({step_name}) has no marked block")
    return pages


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--packages", type=Path, default=ROOT / "target/package")
    parser.add_argument("--journey", choices=list(docs.JOURNEYS), action="append")
    parser.add_argument("--list", action="store_true", help="validate the markers and print the plan")
    parser.add_argument("--pages", action="store_true", help="print the marked pages")
    args = parser.parse_args()
    version = candidate_version()
    pages = validate(docs.load(), version)
    if args.pages:
        print("\n".join(pages))
        return 0
    if args.list:
        for name, journey in docs.JOURNEYS.items():
            print(f"{name}:")
            for page in journey:
                for block in pages[page]:
                    print(f"  {block.where} {block.language} {block.action or 'run'}")
        return 0
    if os.environ.get("GITHUB_ACTIONS") != "true" and os.environ.get("RULLST_FIRST_HOUR") != "1":
        log("This check builds several applications and binds port 3000; it runs in CI "
            "(or set RULLST_FIRST_HOUR=1 on a disposable machine).")
        return 2
    work = Path(tempfile.mkdtemp(prefix="rullst-first-hour-", dir=os.environ.get("RUNNER_TEMP")))
    session = Session(work, args.packages.resolve(), version)
    subprocess.run(["df", "-h", "/"], check=False)
    try:
        with step("cargo-rullst", "install the CLI from the packaged archives"):
            session.install_cli()
    except (Failure, OSError) as error:
        log(f"::error title=First-hour documentation::{error}")
        return 1
    failed = [name for name in args.journey or docs.JOURNEYS if not run_journey(session, name, pages)]
    rows = [f"| {Path(page).name} | {title} | {seconds / 60:.1f} min |" for page, title, seconds in TIMINGS]
    report = "\n".join(["### First-hour documentation", "", "| Page | Step | Time |",
                        "| :--- | :--- | ---: |", *rows,
                        f"| **Total** | | **{sum(t for *_, t in TIMINGS) / 60:.1f} min** |", ""])
    log(report)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as handle:
            handle.write(report + ("\nFailed journeys: " + ", ".join(failed) + "\n" if failed else ""))
    if failed:
        log(f"::error title=First-hour documentation::failed journeys: {', '.join(failed)}")
    return 1 if failed else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except MarkerError as error:
        log(f"::error title=First-hour documentation markers::{error}")
        sys.exit(1)
